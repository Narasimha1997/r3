extern crate log;

use crate::mm::VirtualAddress;
use crate::system::abi;
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

    // sleep this thread for ticks given
    SCHEDULER.lock().sleep_current_thread(ticks);

    // yield
    schedule_yield();
    Ok(0)
}
