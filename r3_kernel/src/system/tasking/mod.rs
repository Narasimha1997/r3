pub mod srbs;
pub mod wait_queue;

extern crate alloc;
extern crate log;
extern crate spin;

use crate::acpi::lapic::LAPICUtils;
use crate::cpu::state::CPURegistersState;
use crate::mm::VirtualAddress;
use crate::system::process::PID;
use crate::system::tasking::srbs::SimpleRoundRobinSchduler;
use crate::system::thread::{Thread, ThreadID};
use crate::system::timer::SystemTimer;

use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug, Clone)]
pub enum SchedAction {
    NoAction,
    CreateChildFork(PID),
}

#[derive(Debug, Clone)]
pub enum ThreadSuspendType {
    Nothing,
    SuspendSleep(usize),
    SuspendWait(PID),
}

#[derive(Debug, Clone)]
pub enum ThreadWakeupType {
    Nothing,
    FromSleep(usize),
    FromWait(PID),
}

/// The trait can be implemented by any schedulable entity.
pub trait Sched {
    /// Should return an empty instance of scheduler.
    /// Use this to initialize the scheduler.
    fn empty() -> Self;

    /// Adds a new thread to internal scheduler's structure.
    /// This can be any structure.
    fn add_new_thread(&mut self, thread: Thread);

    /// Provides a next thread that is runnable on the given core.
    /// which called this function. Note: This function does not actually
    /// run the thread. Instead it just returns a copy of the instance of
    /// that thread.
    fn lease_next_thread(&mut self) -> Option<Thread>;

    /// saves the current context of the thread. Requires the complete
    /// CPU state.
    fn save_current_ctx(&mut self, state: CPURegistersState);

    /// This function is called by the currently running this, calling
    /// this function will automatically make the thread non-schedulable
    /// and it's entry will be removed from everywhere. Including the process
    fn exit(&mut self, code: i64);

    /// this function should return the current thread ID
    /// that called this function, or that was scheduled.
    fn current_tid(&self) -> Option<ThreadID>;
    /// gets the current pid of thread's process
    fn current_pid(&self) -> Option<PID>;

    /// check how many threads can be woken up from wait queue.
    fn check_wakeup(&mut self, wakeup_mode: ThreadWakeupType);

    /// suspend current thread to sleep for x ticks
    fn suspend_thread(&mut self, suspend_type: ThreadSuspendType);

    /// reset current thread
    fn reset_current_thread_stack(&mut self) -> VirtualAddress;
}

lazy_static! {
    pub static ref SCHEDULER: Mutex<SimpleRoundRobinSchduler> =
        Mutex::new(SimpleRoundRobinSchduler::empty());
}

pub fn setup_scheduler() {
    log::info!(
        "Setup scheduler successful, initial thread={:?}",
        SCHEDULER.lock().thread_index
    );
}

#[no_mangle]
/// this function will be called from timer handle.
/// the function will acknowledge the interrupt, selects a thread
/// and initiates it's state.
pub extern "sysv64" fn schedule_handle(state_repr: CPURegistersState) {

    LAPICUtils::eoi();

    SCHEDULER.lock().save_current_ctx(state_repr);

    // if any thread needs to wake up, wake them up.
    SCHEDULER
        .lock()
        .check_wakeup(ThreadWakeupType::FromSleep(1));

    let thread_opt = SCHEDULER.lock().lease_next_thread();
    if thread_opt.is_some() {
        let thread = thread_opt.unwrap();
        log::debug!("Scheduling thread {}", thread.name);
        SystemTimer::next_shot();
        thread.load_state();
    } else {
        // no threads were returned. Load and continue normally.
        SystemTimer::next_shot();
        CPURegistersState::load_state(&state_repr);
    }
}

pub fn schedule_yield() {
    // make an immediate preemption
    // make a manual timer interrupt shot.
    SystemTimer::manual_shot();
}

pub fn handle_exit(thread: &mut Thread) {
    thread.free_stack();
}

// design inspired from: https://github.com/nuta/kerla/blob/main/kernel/process/wait_queue.rs
pub fn wait_until_return<W, R, E>(mut wait_func: W) -> Result<R, E>
where
    W: FnMut() -> Result<Option<R>, E>,
{
    loop {
        // check the return value:
        let func_ret_value = wait_func();
        let wait_ret_value: Option<Result<R, E>> = match func_ret_value {
            Ok(ret_or_pending) => ret_or_pending.and_then(|res| Some(Ok(res))),
            Err(err) => Some(Err(err)),
        };

        if wait_ret_value.is_none() {
            // continue to wait if the task has not returned.
            schedule_yield();
        } else {
            // return the value back to the caller
            return wait_ret_value.unwrap();
        }
    }
}
