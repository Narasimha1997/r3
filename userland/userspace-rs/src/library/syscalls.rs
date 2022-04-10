
pub enum SyscallNumbers {
    Read = 0,
    Write = 1,
}


#[inline(always)]
pub unsafe fn syscall_3(arg0: usize, arg1: usize, arg2: usize, sys_no: usize) -> usize {
    let syscall_result : usize;
    asm!(
        "int 0x80",
        in("rax") sys_no,
        in("rdi") arg0,
        in("rsi") arg1,
        in("rdx") arg2,
        lateout("rax") syscall_result
    );

    syscall_result
}

pub unsafe fn sys_write(fd: usize, buffer: &[u8], size: usize) -> usize {
    let addr = buffer.as_ptr() as usize;
    return syscall_3(fd, addr, size, SyscallNumbers::Write as usize)
}

pub unsafe fn sys_read(fd: usize, buffer: &mut [u8], size: usize) -> usize {
    let addr = buffer.as_ptr() as usize;
    return syscall_3(fd, addr, size, SyscallNumbers::Read as usize)
}