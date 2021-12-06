use crate::system::abi;
use crate::system::timer;

const REALTIME_CLOCK: i32 = 0;
const MONOTONIC_CLOCK: i32 = 1;

pub fn sys_clock_gettime(
    clock_sel: i32,
    timeval_buffer: abi::UserAddress,
) -> Result<isize, abi::Errno> {
    let timeval = match clock_sel {
        // TODO: Handle time with unix epoch
        REALTIME_CLOCK => timer::PosixTimeval::from_ticks(),
        MONOTONIC_CLOCK => timer::PosixTimeval::from_ticks(),
        _ => return Err(abi::Errno::ENOSYS),
    };

    // return this timeval, copy this to userpace buffer provided
    let tval_buffer: &mut timer::PosixTimeval = unsafe { &mut *timeval_buffer.get_mut_ptr() };
    tval_buffer.tv_sec = timeval.tv_sec;
    tval_buffer.tv_usec = timeval.tv_usec;

    // everything ok
    Ok(0)
}
