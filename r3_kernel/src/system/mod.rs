pub mod abi;
pub mod filesystem;
pub mod loader;
pub mod net;
pub mod posix;
pub mod process;
pub mod tasking;
pub mod thread;
pub mod timer;
pub mod utils;

use tasking::Sched;

pub fn init_tasking() {
    process::setup_process_pool();
    tasking::setup_scheduler();
}

pub fn init_fs() {
    filesystem::vfs::setup_fs();
    filesystem::devfs::mount_devfs("/dev/");
}

pub fn probe_filesystems() {
    filesystem::detect::detect_filesystems();
}

pub fn init_networking() {
    net::iface::setup_network_interface();
}

#[inline]
pub fn current_tid() -> Option<thread::ThreadID> {
    tasking::SCHEDULER.lock().current_tid()
}

#[inline]
pub fn current_pid() -> Option<process::PID> {
    tasking::SCHEDULER.lock().current_pid()
}
