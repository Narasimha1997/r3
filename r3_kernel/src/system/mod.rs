pub mod process;
pub mod tasking;
pub mod thread;
pub mod timer;
pub mod filesystem;

pub fn init_tasking() {
    process::setup_process_pool();
    thread::setup_thread_pool();
    tasking::setup_scheduler();
}
