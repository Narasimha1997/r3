use crate::acpi::power;
use crate::drivers::random;
use crate::mm::VirtualAddress;
use crate::system::abi;

use core::ptr;

// these strings are null terminated to make sure they are processed
// properly by usespace C libraries which uses null terminated strings.
const UNAME_STRINGS: &'static [&'static str] = &["LINUX", "", "0.0.1", "r3", "x86_64", ""];

const FIELD_LEN: usize = 65;

pub fn sys_uname(buffer_addr: VirtualAddress) -> Result<isize, abi::Errno> {
    unsafe {
        let mut buffer_ptr = buffer_addr.get_mut_ptr::<u8>();

        for string in UNAME_STRINGS {
            let result = abi::copy_pad_buffer(string.as_bytes(), buffer_ptr, string.len());
            if result.is_err() {
                return Err(result.unwrap_err());
            }

            buffer_ptr = buffer_ptr.add(FIELD_LEN);
        }

        Ok(0)
    }
}

pub fn sys_shutdown() -> Result<isize, abi::Errno> {
    // Byee!
    power::shutdown();

    // this code will never see transistors in CPU
    Ok(0 as isize)
}

pub fn sys_reboot() -> Result<isize, abi::Errno> {
    // try reboot, or else shutdown.
    power::reboot();

    // this code will never see transistors in CPU
    Ok(0 as isize)
}

pub fn sys_getrandom(
    buffer_addr: VirtualAddress,
    buffer_len: usize,
    _flags: u8,
) -> Result<isize, abi::Errno> {

    let buffer_ptr = buffer_addr.get_mut_ptr::<u8>();
    let buffer_slice = unsafe { &mut *ptr::slice_from_raw_parts_mut(buffer_ptr, buffer_len) };

    random::SystemRandomDevice::empty().fill_bytes(buffer_slice);

    Ok(buffer_len as isize)
}
