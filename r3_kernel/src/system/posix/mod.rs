pub mod gettime;
pub mod io;
pub mod mm;
pub mod sched;
pub mod uname;

use crate::mm::VirtualAddress;
use crate::system::abi;
use crate::system::filesystem::POSIXOpenFlags;

const SYSCALL_NO_READ: usize = 0;
const SYSCALL_NO_WRITE: usize = 1;
const SYSCALL_NO_OPEN: usize = 2;
const SYSCALL_NO_CLOSE: usize = 3;
const SYSCALL_NO_LSEEK: usize = 8;
const SYSCALL_NO_BRK: usize = 12;
const SYSCALL_NO_SBRK: usize = 13;
const SYSCALL_NO_IOCTL: usize = 16;
const SYSCALL_NO_YIELD: usize = 42;
const SYSCALL_NO_UNAME: usize = 63;
const SYSCALL_NO_GETTIME: usize = 228;

#[inline]
pub fn dispatch_syscall(
    sys_no: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> isize {
    let syscall_result = match sys_no {
        SYSCALL_NO_GETTIME => {
            // is the pointer null?
            if arg1 == 0 {
                panic!(
                    "Got null pointer - syscall: {} sys_clock_gettime",
                    SYSCALL_NO_GETTIME
                );
            }

            let clock_type = arg0 as i32;
            let userptr = abi::UserAddress::from_u64(arg1 as u64);

            // call the function:
            gettime::sys_clock_gettime(clock_type, userptr)
        }
        SYSCALL_NO_OPEN => {
            if arg0 == 0 {
                panic!("Got null pointer - syscall: {} sys_open", SYSCALL_NO_OPEN);
            }

            let path_res = abi::copy_cstring(VirtualAddress::from_u64(arg0 as u64), 512);
            let open_res = match path_res {
                Err(err_code) => Err(err_code),
                Ok(path) => io::sys_open(&path, POSIXOpenFlags::from_bits_truncate(arg1 as u32)),
            };

            open_res
        }
        SYSCALL_NO_READ => {
            if arg1 == 0 {
                panic!("Got null pointer - syscall: {} sys_read", SYSCALL_NO_OPEN);
            }

            io::sys_read(arg0, VirtualAddress::from_u64(arg1 as u64), arg2)
        }
        SYSCALL_NO_WRITE => {
            if arg1 == 0 {
                panic!("Got null pointer - syscall: {} sys_write", SYSCALL_NO_OPEN);
            }

            io::sys_write(arg0, VirtualAddress::from_u64(arg1 as u64), arg2)
        }
        SYSCALL_NO_LSEEK => io::sys_lseek(arg0, arg1 as u32, arg2 as u8),
        SYSCALL_NO_CLOSE => io::sys_close(arg0),
        SYSCALL_NO_UNAME => {
            if arg0 == 0 {
                panic!("Got null pointer - syscall: {} sys_uname", SYSCALL_NO_UNAME);
            }

            uname::sys_uname(VirtualAddress::from_u64(arg0 as u64))
        }
        SYSCALL_NO_BRK => mm::sys_brk(VirtualAddress::from_u64(arg0 as u64)),
        SYSCALL_NO_SBRK => mm::sys_sbrk(arg0),
        SYSCALL_NO_IOCTL => io::sys_ioctl(arg0, arg1, arg2),
        SYSCALL_NO_YIELD => sched::sys_yield(),
        _ => Err(abi::Errno::ENOSYS),
    };

    if syscall_result.is_err() {
        log::debug!("System Call {} cought error - {:?}", sys_no, syscall_result);
        return syscall_result.unwrap_err() as isize;
    }

    return syscall_result.unwrap() as isize;
}
