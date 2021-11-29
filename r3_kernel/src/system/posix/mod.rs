pub mod gettime;

use crate::system::abi;

pub const SYSCALL_NO_GETTIME: usize = 228;

#[inline]
pub fn dispatch_syscall(sys_no: usize, arg0: usize, arg1: usize, _arg2: usize) -> i32 {
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
        _ => Err(abi::Errno::ENOSYS),
    };

    if syscall_result.is_err() {
        log::debug!("System Call {} cought error - {:?}", sys_no, syscall_result);
        return syscall_result.unwrap_err() as i32;
    }

    return 0;
}
