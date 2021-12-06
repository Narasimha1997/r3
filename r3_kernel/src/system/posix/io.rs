extern crate bitflags;

use crate::mm::VirtualAddress;
use crate::system;
use crate::system::abi;
use crate::system::filesystem::{vfs::FILESYSTEM, FDOps, FSOps, POSIXOpenFlags};
use crate::system::process::{Process, PROCESS_POOL};
use crate::system::utils::ProcessFDPool;

use core::ptr;

// TODO: lot of things needs to be handled properly here.

pub fn sys_open(path: &str, flags: POSIXOpenFlags) -> Result<isize, abi::Errno> {
    let pid = system::current_pid();
    if pid.is_none() {
        log::error!("PID is null.");
        return Err(abi::Errno::EINVAL);
    }

    let mut proc_pool = PROCESS_POOL.lock();
    let proc_ref: &mut Process = proc_pool.get_mut_ref(&pid.unwrap()).unwrap();
    if flags.contains(POSIXOpenFlags::O_CREAT) {
        log::error!("File creation is not implemented yet.");
        return Err(abi::Errno::EINVAL);
    }

    let fd_result = FILESYSTEM.lock().open(&path, flags.bits());
    if fd_result.is_err() {
        log::error!("File {} not found.", path);
        return Err(abi::Errno::EEXIST);
    }

    // create the file-descriptor-index
    let fd = fd_result.unwrap();
    let fd_res = ProcessFDPool::put(&mut proc_ref.proc_data.as_mut().unwrap(), fd);
    if fd_res.is_err() {
        log::error!("Process wide number of open file-descriptors limit has been reached.");
        return Err(abi::Errno::EMFILE);
    }

    let fd_index = fd_res.unwrap();
    Ok(fd_index as isize)
}

pub fn sys_read(
    fd_index: usize,
    buffer_addr: VirtualAddress,
    size: usize,
) -> Result<isize, abi::Errno> {
    let pid = system::current_pid();
    if pid.is_none() {
        log::error!("PID is null.");
        return Err(abi::Errno::EINVAL);
    }

    let mut proc_pool = PROCESS_POOL.lock();
    let proc_ref: &mut Process = proc_pool.get_mut_ref(&pid.unwrap()).unwrap();

    let proc_data = proc_ref.proc_data.as_mut().unwrap();
    let fdref_opt = ProcessFDPool::get_mut(proc_data, fd_index);
    if fdref_opt.is_none() {
        return Err(abi::Errno::EBADF);
    }

    let fdref = fdref_opt.unwrap();

    // TODO: Check if the file was opened for reading
    let mut buffer =
        unsafe { &mut *ptr::slice_from_raw_parts_mut(buffer_addr.get_mut_ptr::<u8>(), size) };
    let read_res = FILESYSTEM.lock().read(&mut fdref.fd, &mut buffer);

    if read_res.is_err() {
        return Err(abi::Errno::EIO);
    }

    // return the number of bytes read
    return Ok(read_res.unwrap() as isize);
}

pub fn sys_write(
    fd_index: usize,
    buffer_addr: VirtualAddress,
    size: usize,
) -> Result<isize, abi::Errno> {
    let pid = system::current_pid();
    if pid.is_none() {
        log::error!("PID is null.");
        return Err(abi::Errno::EINVAL);
    }

    // TODO: make writes possible on all the types of files
    if fd_index == 0 || fd_index > 2 {
        log::error!("As of now, writes are possible on stdout, stderr.");
        return Err(abi::Errno::EINVAL);
    }

    let mut proc_pool = PROCESS_POOL.lock();
    let proc_ref: &mut Process = proc_pool.get_mut_ref(&pid.unwrap()).unwrap();

    let proc_data = proc_ref.proc_data.as_mut().unwrap();
    let fdref_opt = ProcessFDPool::get_mut(proc_data, fd_index);
    if fdref_opt.is_none() {
        return Err(abi::Errno::EBADF);
    }

    let fdref = fdref_opt.unwrap();

    // TODO: Check if the file was opened for writing
    let buffer = unsafe { &*ptr::slice_from_raw_parts(buffer_addr.get_ptr::<u8>(), size) };
    let read_res = FILESYSTEM.lock().write(&mut fdref.fd, &buffer);

    if read_res.is_err() {
        return Err(abi::Errno::EIO);
    }

    // return the number of bytes wrote
    return Ok(read_res.unwrap() as isize);
}

pub fn sys_close(fd_index: usize) -> Result<isize, abi::Errno> {
    let pid = system::current_pid();
    if pid.is_none() {
        log::error!("PID is null.");
        return Err(abi::Errno::EINVAL);
    }

    // call close on the file-system and remove the fd
    let mut proc_pool = PROCESS_POOL.lock();
    let proc_ref: &mut Process = proc_pool.get_mut_ref(&pid.unwrap()).unwrap();

    let proc_data = proc_ref.proc_data.as_mut().unwrap();
    let fdref_opt = ProcessFDPool::get_mut(proc_data, fd_index);
    if fdref_opt.is_none() {
        return Err(abi::Errno::EBADF);
    }

    let fdref = fdref_opt.unwrap();

    // call close
    let close_res = FILESYSTEM.lock().close(&fdref.fd);
    if close_res.is_err() {
        return Err(abi::Errno::EIO);
    }

    // remove from process pool
    let _ = ProcessFDPool::remove(proc_data, fd_index);
    Ok(0)
}
