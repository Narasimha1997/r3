extern crate alloc;
extern crate log;

use crate::mm::VirtualAddress;
use crate::system::abi;
use crate::system::process::{Process, PROCESS_POOL};
use crate::system::tasking::schedule_yield;
use crate::system::tasking::{Sched, ThreadSuspendType, ThreadWakeupType, SCHEDULER};
use crate::system::thread::{ContextType, Thread};
use crate::system::timer::PosixTimeval;
use crate::system::timer::{pause_events, resume_events};

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::{CPURegistersState, SyscallRegsState};
use crate::system::process::PID;

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
    SCHEDULER
        .lock()
        .suspend_thread(ThreadSuspendType::SuspendSleep(ticks));

    // yield
    schedule_yield();
    log::debug!("Returned from sleep!");
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
    // disable interrupts
    pause_events();

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
    resume_events();

    // add this thread to the queue:
    SCHEDULER.lock().add_new_thread(thread);
    // from here, the process will be the child
    // tell the scheduler to run our new process next, by creating a new thread.
    Ok(child_pid.as_u64() as isize)
}

pub fn sys_execvp(path: &str, ist: &mut InterruptStackFrame) -> Result<isize, abi::Errno> {
    pause_events();
    let pid = SCHEDULER.lock().current_pid().unwrap();
    let code_start = PROCESS_POOL.lock().reset_process(&pid, path);
    // reset the thread's internal stack to point to the start from end
    let stack_addr = SCHEDULER.lock().reset_current_thread_stack();

    // set the interrupt stack frame registers
    ist.stack_pointer = stack_addr.as_u64();
    ist.instruction_pointer = code_start.as_u64();
    resume_events();
    Ok(0)
}

pub fn sys_exit(code: i64) -> Result<isize, abi::Errno> {
    pause_events();
    let pid = SCHEDULER.lock().current_pid().unwrap();

    SCHEDULER.lock().exit(code);
    PROCESS_POOL
        .lock()
        .remove_process(&pid, code)
        .expect("Failed to remove process");
    // wakeup waiting threads
    SCHEDULER
        .lock()
        .check_wakeup(ThreadWakeupType::FromWait(pid));

    resume_events();
    schedule_yield();

    // you should never come here!
    Ok(1 as isize)
}

pub fn sys_wait(pid: PID) -> Result<isize, abi::Errno> {
    // suspend the current thread
    SCHEDULER
        .lock()
        .suspend_thread(ThreadSuspendType::SuspendWait(pid));
    // yield the scheduler until next time
    schedule_yield();

    // you will come here once the thread is back in the run queue
    Ok(0 as isize)
}
