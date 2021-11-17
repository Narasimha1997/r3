pub mod process;
pub mod scheduler;
pub mod thread;
pub mod timer;

pub fn init_tasking() {
    process::setup_process_pool();
    thread::setup_thread_pool();
    scheduler::setup_scheduler();
}
