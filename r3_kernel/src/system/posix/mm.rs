extern crate log;

use crate::system;
use crate::system::abi;
use crate::system::process::{Process, PROCESS_POOL};
use crate::system::utils::ProcessHeapAllocator;

use crate::mm::VirtualAddress;

pub fn sys_brk(addr: VirtualAddress) -> Result<isize, abi::Errno> {
    let pid = system::current_pid();
    if pid.is_none() {
        log::error!("PID is null.");
        return Err(abi::Errno::EINVAL);
    }

    let mut proc_pool = PROCESS_POOL.lock();
    let proc_ref: &mut Process = proc_pool.get_mut_ref(&pid.unwrap()).unwrap();

    let brk_res = ProcessHeapAllocator::set_break_at(
        &mut proc_ref.proc_data.as_mut().unwrap(),
        &mut proc_ref.pt_root.as_mut().unwrap(),
        addr,
    );
    if brk_res.is_err() {
        return Err(abi::Errno::ENOMEM);
    }

    Ok(0)
}

pub fn sys_sbrk(size: usize) -> Result<isize, abi::Errno> {
    let pid = system::current_pid();
    if pid.is_none() {
        log::error!("PID is null.");
        return Err(abi::Errno::EINVAL);
    }

    let mut proc_pool = PROCESS_POOL.lock();
    let proc_ref: &mut Process = proc_pool.get_mut_ref(&pid.unwrap()).unwrap();

    let proc_data = proc_ref.proc_data.as_mut().unwrap();

    // get current size
    let current_end_addr = ProcessHeapAllocator::current_end_address(proc_data);
    if size > 0 {
        let sbrk_res =
            ProcessHeapAllocator::expand(proc_data, &mut proc_ref.pt_root.as_mut().unwrap(), size);

        if sbrk_res.is_err() {
            return Err(abi::Errno::ENOMEM);
        }
    }

    Ok(current_end_addr.as_u64() as isize)
}
