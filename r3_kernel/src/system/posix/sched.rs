extern crate log;

use crate::system::abi;
use crate::system::tasking::schedule_yield;

pub fn sys_yield() -> Result<isize, abi::Errno> {
    schedule_yield();
    Ok(1)
}
