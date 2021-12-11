pub mod srbs;

extern crate alloc;
extern crate log;
extern crate spin;

use crate::acpi::lapic::LAPICUtils;
use crate::cpu::state::CPURegistersState;
use crate::system::process::PID;
use crate::system::tasking::srbs::SimpleRoundRobinSchduler;
use crate::system::thread::{Thread, ThreadID};
use crate::system::timer::SystemTimer;

use crate::system::thread::THREAD_POOL;

use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug, Clone)]
pub enum SchedAction {
    NoAction,
    CreateChildFork(PID),
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
    fn exit(&mut self, code: u64);

    /// this function should return the current thread ID
    /// that called this function, or that was scheduled.
    fn current_tid(&self) -> Option<ThreadID>;
    /// gets the current pid of thread's process
    fn current_pid(&self) -> Option<PID>;

    /// check how many threads can be woken up from wait queue.
    fn check_wakeup(&mut self);

    /// suspend current thread to sleep for x ticks
    fn sleep_current_thread(&mut self, n_ticks: usize);
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
    // eoi:
    LAPICUtils::eoi();

    SCHEDULER.lock().save_current_ctx(state_repr);

    // if any thread needs to wake up, wake them up.
    SCHEDULER.lock().check_wakeup();

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

    THREAD_POOL
        .lock()
        .remove_thread(&thread.thread_id)
        .expect("Incosistent scheduler state, failed to remove thread.");
}

// calls the exit with a code
pub fn exit(code: u64) {
    SCHEDULER.lock().exit(code);
}
