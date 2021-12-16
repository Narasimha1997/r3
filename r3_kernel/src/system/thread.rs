extern crate alloc;
extern crate log;
extern crate spin;

use crate::cpu::{
    mmu, segments, state::bootstrap_kernel_thread, state::CPURegistersState, syscall,
};
use crate::mm::{stack::STACK_ALLOCATOR, stack::STACK_SIZE, PhysicalAddress, VirtualAddress};
use crate::system::process::{PID, PROCESS_POOL};
use crate::system::tasking::{Sched, SCHEDULER};

use crate::system::utils;

use alloc::{boxed::Box, string::String};
use core::sync::atomic::{AtomicU64, Ordering};

pub type ThreadFn = fn();

static CURRENT_TID: AtomicU64 = AtomicU64::new(0);

pub fn new_tid() -> ThreadID {
    let current = CURRENT_TID.load(Ordering::SeqCst);
    if current + 1 == u64::max_value() {
        panic!("Could not create thread, Out of TIDs");
    }

    CURRENT_TID.store(current + 1, Ordering::SeqCst);
    ThreadID(current)
}

#[derive(Debug, Clone)]
/// Contains the initial state required enough to spin off a thread.
pub struct InitialStateContainer {
    pub rip_address: VirtualAddress,
    pub cr3_base: u64,
    pub stack_end: VirtualAddress,
}

#[derive(Debug, Clone)]
/// Either a saved state of complete registers or an initial state.
pub enum ContextType {
    SavedContext(CPURegistersState),
    InitContext(InitialStateContainer),
}

#[derive(Clone, Debug, Copy)]
pub struct ThreadID(u64);

impl ThreadID {
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum ThreadState {
    Running,
    Waiting,
    // TODO: Define more states later.
}

#[derive(Debug)]
pub enum ThreadError {
    NoPID,
    OutOfStacks,
    UnknownThreadID,
}

pub struct Context;

impl Context {
    #[inline]
    /// returns a default state with all zeroed values
    pub fn init() -> CPURegistersState {
        CPURegistersState::default()
    }
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub thread_id: ThreadID,
    pub parent_pid: PID,
    pub context: Box<ContextType>,
    pub name: String,
    pub state: ThreadState,
    pub sched_count: u64,
    pub stack_start: VirtualAddress,
    pub is_user: bool,
    pub cr3: u64,
    pub syscall_stack_start: Option<VirtualAddress>,
}

impl Thread {
    pub fn new(pid: PID, name: String) -> Result<Self, ThreadError> {
        let mut proc_lock = PROCESS_POOL.lock();

        let parent_proc_opt = proc_lock.get_mut_ref(&pid);

        if parent_proc_opt.is_none() {
            return Err(ThreadError::NoPID);
        }

        let parent_proc = parent_proc_opt.unwrap();
        if parent_proc.proc_data.is_none() {
            panic!("Cannot run thread on empty user process layout.");
        }

        if !parent_proc.is_usermode() {
            panic!("Kernel threads cannot load and run ELF binaries.");
        }

        let mut proc_data = parent_proc.proc_data.as_mut().unwrap();

        // allocate a stack
        let stack_start = utils::ProcessStackManager::allocate_stack(
            &mut proc_data,
            parent_proc.pt_root.as_mut().unwrap().as_mut(),
            true,
        )
        .expect("Failed to allocate stack for user thread.");

        let syscall_stack_start = utils::ProcessStackManager::allocate_syscall_stack(
            &mut proc_data,
            parent_proc.pt_root.as_mut().unwrap().as_mut(),
            0,
        )
        .expect("Failed to allocate syscall stack");

        // get the entrypoint:
        let entrypoint = proc_data.code_entry;

        let tid = new_tid();
        parent_proc.add_thread(tid.clone());

        // init context
        let init_context = InitialStateContainer {
            cr3_base: parent_proc.cr3,
            rip_address: entrypoint,
            stack_end: VirtualAddress::from_u64(stack_start.as_u64() + STACK_SIZE as u64),
        };

        log::debug!(
            "Initialized context for new thread
            thread_id={}, page_table=0x{:x}, rip=0x{:x},
            stack_end=0x{:x}",
            tid.as_u64(),
            init_context.cr3_base,
            init_context.rip_address.as_u64(),
            init_context.stack_end.as_u64()
        );

        let context = ContextType::InitContext(init_context);

        Ok(Thread {
            is_user: true,
            parent_pid: parent_proc.pid.clone(),
            context: Box::new(context),
            name,
            thread_id: tid,
            state: ThreadState::Waiting,
            sched_count: 0,
            stack_start: stack_start,
            cr3: parent_proc.cr3,
            syscall_stack_start: Some(syscall_stack_start),
        })
    }

    pub fn new_from_parent(
        name: String,
        pid: PID,
        state: &ContextType,
    ) -> Result<Self, ThreadError> {
        let mut proc_lock = PROCESS_POOL.lock();

        let child = proc_lock.get_mut_ref(&pid).unwrap();

        let rsp = match &state {
            ContextType::InitContext(ctx) => ctx.stack_end.as_u64(),
            ContextType::SavedContext(ctx) => ctx.rsp,
        };

        // allocate a new stack and copy the parent stack:
        let stack_start = utils::ProcessStackManager::allocate_and_clone(
            &mut child.proc_data.as_mut().unwrap(),
            &mut child.pt_root.as_mut().unwrap(),
            rsp,
        )
        .expect("Failed to allocate stack for new thread.");

        let syscall_stack_start = utils::ProcessStackManager::allocate_syscall_stack(
            &mut child.proc_data.as_mut().unwrap(),
            &mut child.pt_root.as_mut().unwrap(),
            0,
        )
        .expect("Failed to allocate syscall stack");

        let tid = new_tid();
        child.add_thread(tid.clone());

        Ok(Thread {
            is_user: true,
            parent_pid: pid,
            context: Box::new(state.clone()),
            name,
            thread_id: tid,
            state: ThreadState::Waiting,
            sched_count: 0,
            stack_start: stack_start,
            cr3: child.cr3,
            syscall_stack_start: Some(syscall_stack_start),
        })
    }

    pub fn new_from_function(
        pid: PID,
        name: String,
        function_addr: VirtualAddress,
    ) -> Result<Self, ThreadError> {
        // get parent process reference:
        let mut proc_lock = PROCESS_POOL.lock();

        let parent_proc = proc_lock.get_mut_ref(&pid);

        if parent_proc.is_none() {
            return Err(ThreadError::NoPID);
        }

        // function exists, now check if it is a user process
        let proc = parent_proc.unwrap();

        let stack: VirtualAddress;
        let func_addr: VirtualAddress;

        if proc.is_usermode() {
            let stack_alloc_result = STACK_ALLOCATOR.lock().alloc_stack();
            if stack_alloc_result.is_err() {
                panic!("Out of stack memory. Failed to allocate memory for thread.");
            }
            // allocate a new stack for the kernel
            stack = utils::map_user_stack(
                stack_alloc_result.unwrap(),
                proc.threads.len(),
                proc.pt_root.as_mut().unwrap().as_mut(),
            );

            // map function to given address:
            func_addr =
                utils::map_user_code(function_addr, proc.pt_root.as_mut().unwrap().as_mut());
        } else {
            let stack_alloc_result = STACK_ALLOCATOR.lock().alloc_stack();
            if stack_alloc_result.is_err() {
                panic!("Out of stack memory. Failed to allocate memory for thread.");
            }
            stack = stack_alloc_result.unwrap();
            func_addr = function_addr;
        }

        let parent_cr3 = proc.cr3;

        let init_context = InitialStateContainer {
            cr3_base: parent_cr3,
            rip_address: func_addr,
            stack_end: VirtualAddress::from_u64(stack.as_u64() + STACK_SIZE as u64),
        };

        let tid = new_tid();
        proc.add_thread(tid.clone());

        log::debug!(
            "Initialized context for new thread
            thread_id={}, page_table=0x{:x}, rip=0x{:x},
            stack_end=0x{:x}",
            tid.as_u64(),
            init_context.cr3_base,
            init_context.rip_address.as_u64(),
            init_context.stack_end.as_u64()
        );

        // create a new state:
        let context = ContextType::InitContext(init_context);

        Ok(Thread {
            is_user: proc.is_usermode(),
            parent_pid: pid,
            context: Box::new(context),
            name,
            thread_id: tid,
            state: ThreadState::Waiting,
            sched_count: 0,
            stack_start: stack,
            cr3: parent_cr3,
            syscall_stack_start: None,
        })
    }

    #[inline]
    pub fn free_stack(&self) {
        STACK_ALLOCATOR
            .lock()
            .free_stack(self.stack_start)
            .expect("Failed to free stack after thread exit");
    }

    #[inline]
    pub fn reset_stack(&mut self) -> VirtualAddress {
        // reset the stack:
        utils::ProcessStackManager::reset_stack(self.stack_start);
        VirtualAddress::from_u64(self.stack_start.as_u64() + STACK_SIZE as u64)
    }

    #[inline]
    pub fn load_state(&self) {
        match self.context.as_ref() {
            ContextType::InitContext(ctx) => {
                // initial context, create a new context object:
                let (code_sel, data_sel) = if self.is_user {
                    (segments::get_user_cs().0, segments::get_user_ds().0)
                } else {
                    (segments::get_kernel_cs().0, segments::get_kernel_ds().0)
                };

                if self.syscall_stack_start.is_some() {
                    // load this custom stack:
                    let stack_end = self.syscall_stack_start.unwrap().as_u64()
                        + utils::THREAD_SYSCALL_STACK_SIZE;
                    // set this stack
                    syscall::set_syscall_stack(stack_end);
                } else {
                    syscall::set_default_syscall_stack();
                }

                mmu::set_page_table_address(PhysicalAddress::from_u64(ctx.cr3_base));

                mmu::reload_flush();

                bootstrap_kernel_thread(
                    ctx.stack_end.as_u64(),
                    ctx.rip_address.as_u64(),
                    code_sel,
                    data_sel,
                )
            }
            ContextType::SavedContext(ctx) => {
                // load page tables:
                if self.syscall_stack_start.is_some() {
                    // load this custom stack:
                    let stack_end = self.syscall_stack_start.unwrap().as_u64()
                        + utils::THREAD_SYSCALL_STACK_SIZE;
                    // set this stack
                    syscall::set_syscall_stack(stack_end);
                } else {
                    syscall::set_default_syscall_stack();
                }
                mmu::set_page_table_address(PhysicalAddress::from_u64(self.cr3));
                CPURegistersState::load_state(&ctx)
            }
        }
    }
}

pub fn new_from_function(
    pid: &PID,
    name: String,
    function_addr: VirtualAddress,
) -> Result<ThreadID, ThreadError> {
    let th_res = Thread::new_from_function(pid.clone(), name, function_addr);
    if th_res.is_err() {
        return Err(th_res.unwrap_err());
    }

    let thread = th_res.unwrap();
    
    SCHEDULER.lock().add_new_thread(thread.clone());
    Ok(thread.thread_id)
}

pub fn new_main_thread(pid: &PID, name: String) -> Result<ThreadID, ThreadError> {
    let th_res = Thread::new(pid.clone(), name);
    if th_res.is_err() {
        return Err(th_res.unwrap_err());
    }
    let thread = th_res.unwrap();
    let tid = thread.thread_id;

    SCHEDULER.lock().add_new_thread(thread.clone());
    Ok(tid)
}
