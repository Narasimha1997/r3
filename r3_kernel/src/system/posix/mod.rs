pub mod gettime;
pub mod io;
pub mod mm;
pub mod sched;
pub mod uname;

use crate::mm::VirtualAddress;
use crate::system::abi;
use crate::system::filesystem::POSIXOpenFlags;
use crate::system::process::PID;

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::SyscallRegsState;

const SYSCALL_NO_READ: usize = 0;
const SYSCALL_NO_WRITE: usize = 1;
const SYSCALL_NO_OPEN: usize = 2;
const SYSCALL_NO_CLOSE: usize = 3;
const SYSCALL_NO_EXIT: usize = 4;
const SYSCALL_NO_FSTAT: usize = 5;
const SYSCALL_NO_LSTAT: usize = 6;
const SYSCALL_NO_LSEEK: usize = 8;
const SYSCALL_NO_PID: usize = 9;
const SYSCALL_NO_PPID: usize = 10;
const SYSCALL_NO_FORK: usize = 11;
const SYSCALL_NO_BRK: usize = 12;
const SYSCALL_NO_SBRK: usize = 13;
const SYSCALL_NO_IOCTL: usize = 16;
const SYSCALL_NO_YIELD: usize = 42;
const SYSCALL_NO_TID: usize = 43;
const SYSCALL_NO_SLEEP: usize = 46;
const SYSCALL_NO_WAIT: usize = 47;
const SYSCALL_NO_EXECVP: usize = 59;
const SYSCALL_NO_UNAME: usize = 63;
const SYSCALL_NO_GETTIME: usize = 228;

#[inline]
pub fn dispatch_syscall(regs: &mut SyscallRegsState, frame: &mut InterruptStackFrame) -> isize {
    // get basic arguments:
    let sys_no = regs.rax as usize;
    let arg0 = regs.rdi as usize;
    let arg1 = regs.rsi as usize;
    let arg2 = regs.rdx as usize;

    log::debug!(
        "SYSCALL: sys_no={}, arg0=0x{:x}, arg1=0x{:x}, arg2=0x{:x}",
        sys_no,
        arg0,
        arg1,
        arg2
    );

    let syscall_result = match sys_no {
        SYSCALL_NO_GETTIME => {
            // is the pointer null?
            let res = if !abi::is_in_userspace(arg1 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                let clock_type = arg0 as i32;
                let userptr = abi::UserAddress::from_u64(arg1 as u64);
                // call the function:
                gettime::sys_clock_gettime(clock_type, userptr)
            };

            res
        }
        SYSCALL_NO_OPEN => {
            let res = if !abi::is_in_userspace(arg0 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                let path_res = abi::copy_cstring(VirtualAddress::from_u64(arg0 as u64), 512);
                let open_res = match path_res {
                    Err(err_code) => Err(err_code),
                    Ok(path) => {
                        io::sys_open(&path, POSIXOpenFlags::from_bits_truncate(arg1 as u32))
                    }
                };

                open_res
            };
            res
        }
        SYSCALL_NO_READ => {
            let res = if !abi::is_in_userspace(arg1 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                io::sys_read(arg0, VirtualAddress::from_u64(arg1 as u64), arg2)
            };
            res
        }
        SYSCALL_NO_WRITE => {
            let res = if !abi::is_in_userspace(arg1 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                io::sys_write(arg0, VirtualAddress::from_u64(arg1 as u64), arg2)
            };
            res
        }
        SYSCALL_NO_LSEEK => io::sys_lseek(arg0, arg1 as u32, arg2 as u8),
        SYSCALL_NO_CLOSE => io::sys_close(arg0),
        SYSCALL_NO_EXIT => sched::sys_exit(arg0 as i64),
        SYSCALL_NO_FSTAT => {
            let res = if !abi::is_in_userspace(arg1 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                io::sys_fstat(arg0, VirtualAddress::from_u64(arg1 as u64))
            };
            res
        }
        SYSCALL_NO_LSTAT => {
            let res = if !abi::is_in_userspace(arg1 as u64) || !abi::is_in_userspace(arg0 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                let path_res = abi::copy_cstring(VirtualAddress::from_u64(arg0 as u64), 512);
                let open_res = match path_res {
                    Err(err_code) => Err(err_code),
                    Ok(path) => io::sys_lstat(&path, VirtualAddress::from_u64(arg1 as u64)),
                };

                open_res
            };

            res
        }
        SYSCALL_NO_UNAME => {
            let res = if !abi::is_in_userspace(arg0 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                uname::sys_uname(VirtualAddress::from_u64(arg0 as u64))
            };

            res
        }
        SYSCALL_NO_BRK => mm::sys_brk(VirtualAddress::from_u64(arg0 as u64)),
        SYSCALL_NO_SBRK => mm::sys_sbrk(arg0),
        SYSCALL_NO_IOCTL => io::sys_ioctl(arg0, arg1, arg2),
        SYSCALL_NO_YIELD => sched::sys_yield(),
        SYSCALL_NO_SLEEP => {
            let res = if !abi::is_in_userspace(arg0 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                sched::sys_sleep_us(VirtualAddress::from_u64(arg0 as u64))
            };

            res
        }
        SYSCALL_NO_WAIT => sched::sys_wait(PID::new(arg0 as u64)),
        SYSCALL_NO_PID => sched::sys_pid(),
        SYSCALL_NO_PPID => sched::sys_ppid(),
        SYSCALL_NO_TID => sched::sys_tid(),
        SYSCALL_NO_FORK => sched::sys_fork(&regs, &frame),
        SYSCALL_NO_EXECVP => {
            let res = if !abi::is_in_userspace(arg0 as u64) {
                Err(abi::Errno::EFAULT)
            } else {
                let path_res = abi::copy_cstring(VirtualAddress::from_u64(arg0 as u64), 512);
                let execvp_res = match path_res {
                    Ok(path) => sched::sys_execvp(&path, frame),
                    Err(err_code) => Err(err_code),
                };

                execvp_res
            };

            res
        }
        _ => Err(abi::Errno::ENOSYS),
    };

    if syscall_result.is_err() {
        log::debug!("System Call {} cought error - {:?}", sys_no, syscall_result);
        return syscall_result.unwrap_err() as isize;
    }

    return syscall_result.unwrap() as isize;
}
