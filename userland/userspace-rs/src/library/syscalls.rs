use core::arch::asm;
use crate::library::types::{UTSName, FStatInfo};

pub enum SyscallNumbers {
    Read = 0,
    Write = 1,
    LStat = 6,
    Shutdown = 48,
    Uname = 63,
}

#[inline(always)]
unsafe fn syscall_0(sys_no: usize) -> usize {
    let syscall_result: usize;
    asm!(
        "int 0x80",
        in("rax") sys_no,
        lateout("rax") syscall_result
    );

    syscall_result
}

#[inline(always)]
unsafe fn syscall_1(arg0: usize, sys_no: usize) -> usize {
    let syscall_result: usize;
    asm!(
        "int 0x80",
        in("rax") sys_no,
        in("rdi") arg0,
        lateout("rax") syscall_result
    );

    syscall_result
}

#[inline(always)]
unsafe fn syscall_2(arg0: usize, arg1: usize, sys_no: usize) -> usize {
    let syscall_result: usize;
    asm!(
        "int 0x80",
        in("rax") sys_no,
        in("rdi") arg0,
        in("rsi") arg1,
        lateout("rax") syscall_result,
    );

    syscall_result
}

#[inline(always)]
unsafe fn syscall_3(arg0: usize, arg1: usize, arg2: usize, sys_no: usize) -> usize {
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
    syscall_3(fd, addr, size, SyscallNumbers::Write as usize)
}

pub unsafe fn sys_read(fd: usize, buffer: &mut [u8], size: usize) -> usize {
    let addr = buffer.as_ptr() as usize;
    syscall_3(fd, addr, size, SyscallNumbers::Read as usize)
}

pub unsafe fn sys_uname(uts: &mut UTSName) -> usize {
    let addr = (uts as *const _) as usize;
    syscall_1(addr, SyscallNumbers::Uname as usize)
} 

pub unsafe fn sys_shutdown() -> usize {
    syscall_0(SyscallNumbers::Shutdown as usize)
}

pub unsafe fn sys_lstat(path: &str, stat: &mut FStatInfo) -> usize {
    let path_addr = path.as_ptr() as usize;
    let stat_addr = (stat as *const _) as usize;

    syscall_2(path_addr, stat_addr, SyscallNumbers::LStat as usize)
}