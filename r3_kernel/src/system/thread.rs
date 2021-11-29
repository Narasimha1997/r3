extern crate alloc;
extern crate log;
extern crate spin;

use crate::cpu::{mmu, segments, state::bootstrap_kernel_thread, state::CPURegistersState};
use crate::mm::paging::{KernelVirtualMemoryManager, Page, PageEntryFlags, VirtualMemoryManager};
use crate::mm::phy::Frame;
use crate::mm::{stack::STACK_ALLOCATOR, stack::STACK_SIZE, VirtualAddress};
use crate::mm::{Alignment, MemorySizes, PhysicalAddress};
use crate::system::process::{PID, PROCESS_POOL};
use crate::system::tasking::{Sched, SCHEDULER};

use alloc::{boxed::Box, collections::BTreeMap, string::String};
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

/// Area in which user code will be allocated
pub const USER_CODE_ADDRESS: u64 = 0x100000000;
/// Area in which user stack will be allocated
pub const USER_STACK_ADDRESS: u64 = 0x200000000;

const NEW_RFLAG: u64 = 0x204;

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

    #[inline]
    /// Takes the state and fills it with values suitable to run
    /// a kernel thread. (Should be used only for initial context)
    pub fn fill_for_kthread(
        state: &mut CPURegistersState,
        stack_end: VirtualAddress,
        func_addr: VirtualAddress,
    ) {
        //
        let kernel_cs = segments::get_kernel_cs();

        // set code segment selector, because the kernel code lies
        // in the same segment.
        state.cs = kernel_cs.0 as u64;
        // ss will not be ignored in x86_64
        state.ss = 0;

        // make rsp point to end of the kernel stack
        // because the stack grows downwards.
        state.rsp = stack_end.as_u64();

        // make rip point to the start of function instructions.
        state.rip = func_addr.as_u64();

        state.rflags = NEW_RFLAG;
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
}

impl Thread {
    #[inline]
    fn map_sys_stack(
        stack_addr: VirtualAddress,
        n_current_threads: usize,
        proc_vmm: &mut VirtualMemoryManager,
    ) -> VirtualAddress {
        // maps the stack address to user code's stack location
        // using huge pages
        let new_stack_address =
            VirtualAddress::from_u64(USER_STACK_ADDRESS + (n_current_threads * STACK_SIZE) as u64);
        // map the stack to it's virtual address:
        let stack_phy_address = KernelVirtualMemoryManager::pt().translate(stack_addr);
        if stack_phy_address.is_none() {
            panic!("Incosistent memory state while allocating thread.");
        }

        // map this physical address to given new virtual address as a 2MiB Page
        let res = proc_vmm.map_huge_page(
            Page::from_address(new_stack_address),
            Frame::from_address(stack_phy_address.unwrap()),
            PageEntryFlags::user_hugepage_flags(),
        );
        if res.is_err() {
            panic!("{:?}", res);
        }
        return new_stack_address;
    }

    #[inline]
    fn map_fn_code(
        func_addr: VirtualAddress,
        proc_vmm: &mut VirtualMemoryManager,
    ) -> VirtualAddress {
        let func_phy_addr = KernelVirtualMemoryManager::pt()
            .translate(func_addr)
            .unwrap();

        let base_aligned_addr =
            Alignment::align_down(func_phy_addr.as_u64(), MemorySizes::OneKiB as u64 * 4);

        log::info!(
            "Func phy addr: 0x{:x}, aligned: 0x{:x}",
            func_phy_addr.as_u64(),
            base_aligned_addr
        );

        let offset = func_phy_addr.as_u64() - base_aligned_addr;
        let code_base_addr = VirtualAddress::from_u64(USER_CODE_ADDRESS);
        // map this to virtual memory region:
        proc_vmm
            .map_page(
                Page::from_address(code_base_addr),
                Frame::from_address(PhysicalAddress::from_u64(base_aligned_addr)),
                PageEntryFlags::user_flags(),
            )
            .expect("Failed to map codebase address for user thread.");
        let gaurd_frame = base_aligned_addr + (4 * MemorySizes::OneKiB as u64);

        // map this extra page:
        proc_vmm
            .map_page(
                Page::from_address(VirtualAddress::from_u64(
                    code_base_addr.as_u64() + (4 * MemorySizes::OneKiB as u64),
                )),
                Frame::from_address(PhysicalAddress::from_u64(gaurd_frame)),
                PageEntryFlags::user_flags(),
            )
            .expect("Gaurd page allocation error");
        // return the code address:
        VirtualAddress::from_u64(code_base_addr.as_u64() + offset)
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
            stack = Self::map_sys_stack(
                stack_alloc_result.unwrap(),
                proc.threads.len(),
                proc.pt_root.as_mut().unwrap().as_mut(),
            );

            // map function to given address:
            func_addr = Self::map_fn_code(function_addr, proc.pt_root.as_mut().unwrap().as_mut());
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
    pub fn load_state(&self) {
        match self.context.as_ref() {
            ContextType::InitContext(ctx) => {
                // initial context, create a new context object:
                let (code_sel, data_sel) = if self.is_user {
                    (segments::get_user_cs().0, segments::get_user_ds().0)
                } else {
                    (segments::get_kernel_cs().0, segments::get_kernel_ds().0)
                };

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
                mmu::set_page_table_address(PhysicalAddress::from_u64(self.cr3));
                CPURegistersState::load_state(&ctx)
            }
        }
    }
}

/// ThreadPool: Stores all the thread irrespective of their states
/// this structure serves as a book-keeper for threads.
pub struct ThreadPool {
    /// Number of threads currently under book-keeping.
    pub n_threads: usize,
    /// Number of threads currently
    pub pool_map: BTreeMap<u64, Thread>,
}

impl ThreadPool {
    pub fn new() -> Self {
        ThreadPool {
            n_threads: 0,
            pool_map: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn add_thread(&mut self, thread: Thread) {
        let thread_id = thread.thread_id.as_u64();
        self.pool_map.insert(thread_id, thread);
        self.n_threads += 1;
    }

    #[inline]
    pub fn has_thread(&mut self, tid: &ThreadID) -> bool {
        self.pool_map.contains_key(&tid.as_u64())
    }

    #[inline]
    pub fn remove_thread(&mut self, tid: &ThreadID) -> Result<(), ThreadError> {
        let res = self.pool_map.remove(&tid.as_u64());
        if res.is_none() {
            return Err(ThreadError::UnknownThreadID);
        }

        self.n_threads -= 1;
        Ok(())
    }

    #[inline]
    pub fn get_ref(&self, tid: &ThreadID) -> Option<&Thread> {
        self.pool_map.get(&tid.as_u64())
    }

    #[inline]
    pub fn get_mut_ref(&mut self, tid: &ThreadID) -> Option<&mut Thread> {
        self.pool_map.get_mut(&tid.as_u64())
    }

    pub fn debug_dump_tids(&self) {
        for (tid, th) in &self.pool_map {
            log::debug!("{}:{}", th.name, tid);
        }
    }
}

lazy_static! {
    pub static ref THREAD_POOL: Mutex<ThreadPool> = Mutex::new(ThreadPool::new());
}

pub fn setup_thread_pool() {
    log::info!(
        "Thread pool setup successfull, n_threads={}",
        &THREAD_POOL.lock().n_threads
    );
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
    let tid = thread.thread_id;

    THREAD_POOL.lock().add_thread(thread);
    Ok(tid)
}

pub fn run_thread(tid: &ThreadID) {
    let mut pool_lock = THREAD_POOL.lock();
    let thread_obj = pool_lock.get_mut_ref(tid);

    if thread_obj.is_none() {
        panic!("Invalid thread tid={}", tid.as_u64());
    }

    SCHEDULER.lock().add_new_thread(thread_obj.unwrap().clone());
}
