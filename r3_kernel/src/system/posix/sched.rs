extern crate alloc;
extern crate log;

use crate::mm::VirtualAddress;
use crate::system::abi;
use crate::system::process::{Process, PROCESS_POOL};
use crate::system::tasking::schedule_yield;
use crate::system::tasking::{Sched, SCHEDULER};
use crate::system::thread::{ContextType, Thread};
use crate::system::timer::PosixTimeval;

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::{CPURegistersState, SyscallRegsState};

use alloc::format;

pub fn sys_yield() -> Result<isize, abi::Errno> {
    schedule_yield();
    Ok(1)
}

// TODO: check for overflows
pub fn sys_sleep_us(timeval_addr: VirtualAddress) -> Result<isize, abi::Errno> {
    let timeval_buffer: &PosixTimeval = unsafe { &*timeval_addr.get_ptr() };
    let ticks = timeval_buffer.to_ticks();

    // sleep this thread for {ticks given
    SCHEDULER.lock().sleep_current_thread(ticks);

    // yield
    schedule_yield();
    Ok(0)
}

pub fn sys_pid() -> Result<isize, abi::Errno> {
    let current_pid = SCHEDULER.lock().current_pid().unwrap();
    Ok(current_pid.as_u64() as isize)
}

pub fn sys_ppid() -> Result<isize, abi::Errno> {
    let current_pid = SCHEDULER.lock().current_pid().unwrap();
    let ppid = PROCESS_POOL
        .lock()
        .get_mut_ref(&current_pid)
        .unwrap()
        .ppid
        .as_u64();
    Ok(ppid as isize)
}

pub fn sys_tid() -> Result<isize, abi::Errno> {
    let current_tid = SCHEDULER.lock().current_tid().unwrap().as_u64();
    Ok(current_tid as isize)
}

pub fn sys_fork(regs: &SyscallRegsState, frame: &InterruptStackFrame) -> Result<isize, abi::Errno> {
    let parent_pid = SCHEDULER.lock().current_pid().unwrap();

    // spawn a new process, which is the child
    let child = Process::create_from_parent(&parent_pid);
    let child_pid = child.pid.clone();
    // a new process has been created, which has the same data as that of the parent.
    // register this process
    PROCESS_POOL.lock().add_process(child);

    // prepare a new state:
    let mut state: CPURegistersState = CPURegistersState::default();

    // load registers from syscall state
    state.r11 = regs.r11;
    state.r10 = regs.r10;
    state.r9 = regs.r9;
    state.r8 = regs.r8;
    state.rdi = regs.rdi;
    state.rsi = regs.rsi;
    state.rdx = regs.rdx;
    state.rcx = regs.rcx;

    // set rax to the child pid
    state.rax = child_pid.as_u64();

    // load registers from interrupt frame:
    state.rip = frame.instruction_pointer;
    state.ss = frame.stack_segment;
    state.rsp = frame.stack_pointer;
    state.rflags = frame.cpu_flags;
    state.cs = frame.code_segment;

    // create a thread from this state:
    let thread = Thread::new_from_parent(
        format!("th_{}_{}", parent_pid.as_u64(), child_pid.as_u64()),
        child_pid.clone(),
        &ContextType::SavedContext(state),
    )
    .expect("Failed to create child thread");

    // add this thread to the queue:
    SCHEDULER.lock().add_new_thread(thread);

    // from here, the process will be the child
    // tell the scheduler to run our new process next, by creating a new thread.
    Ok(child_pid.as_u64() as isize)
}
