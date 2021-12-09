extern crate log;

use crate::mm::VirtualAddress;
use crate::system::abi;
use crate::system::process::PROCESS_POOL;
use crate::system::tasking::schedule_yield;
use crate::system::tasking::{Sched, SCHEDULER};
use crate::system::timer::PosixTimeval;

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
