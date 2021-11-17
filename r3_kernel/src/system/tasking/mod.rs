pub mod srbs;

extern crate alloc;
extern crate log;
extern crate spin;

use crate::acpi::lapic::LAPICUtils;
use crate::cpu::state::CPURegistersState;
use crate::system::tasking::srbs::SimpleRoundRobinSchduler;
use crate::system::thread::Thread;
use crate::system::timer::SystemTimer;

use lazy_static::lazy_static;
use spin::Mutex;

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

    /// saves the current context of the thread.
    fn save_current_ctx(&mut self, state: CPURegistersState);
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
    let thread_opt = SCHEDULER.lock().lease_next_thread();
    if thread_opt.is_some() {
        SystemTimer::next_shot();
        thread_opt.unwrap().load_state();
    } else {
        // no threads were returned. Load and continue normally.
        SystemTimer::next_shot();
        CPURegistersState::load_state(&state_repr);
    }
}
