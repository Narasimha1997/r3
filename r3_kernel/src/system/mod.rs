pub mod filesystem;
pub mod process;
pub mod tasking;
pub mod thread;
pub mod timer;

pub fn init_tasking() {
    process::setup_process_pool();
    thread::setup_thread_pool();
    tasking::setup_scheduler();
}

pub fn init_fs() {
    filesystem::vfs::setup_fs();
    filesystem::devfs::mount_devfs("/dev/");
}
