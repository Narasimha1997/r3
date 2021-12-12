extern crate alloc;
extern crate log;

use crate::acpi::lapic::LAPICUtils;

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::SyscallRegsState;

use crate::mm::VirtualAddress;

use crate::system::posix::dispatch_syscall;
use alloc::{string::String, vec};
use core::{ptr, str};

// C numerical types
pub type CInt16 = i16;
pub type CInt32 = i32;
pub type CInt64 = i64;
pub type CUint32 = u32;
pub type CUint64 = u64;
pub type CInt = i32;
pub type CUint = u32;
pub type CShort = i16;
pub type CLong = i64;
pub type CULong = u64;

// clock types:
pub type CTime = CInt64;
pub type CSubSeconds = CInt64;
pub type CClockID = CInt;

#[derive(Debug, Clone)]
#[repr(i32)]
pub enum Errno {
    EIO = 5,
    EBADF = 9,
    ENOMEM = 12,
    EFAULT = 14,
    EEXIST = 17,
    ENOSYS = 32,
    EINVAL = 22,
    EMFILE = 24,
    ENOTTY = 25,
    ENAMETOOLONG = 63,
}

pub type UserAddress = VirtualAddress;

#[no_mangle]
pub extern "sysv64" fn syscall_handler(
    frame: &mut InterruptStackFrame,
    regs: &mut SyscallRegsState,
) {

    let result = dispatch_syscall(regs, frame);

    regs.rax = result as u64;
    LAPICUtils::eoi();
}

/// copies a c-like string to rust String on kernel heap.
pub fn copy_cstring(source: VirtualAddress, max_len: usize) -> Result<String, Errno> {
    let mut buffer = vec![0; max_len];

    // copy until null terminated
    let mut c_ptr = source.get_ptr::<u8>();
    let mut iter = 0;

    // good old C
    unsafe {
        while *c_ptr != b'\0' {
            buffer[iter] = *c_ptr;
            iter = iter + 1;
            if iter >= 512 {
                return Err(Errno::ENAMETOOLONG);
            }
            c_ptr = c_ptr.add(1);
        }
    }

    let rust_str_res = str::from_utf8(&buffer[0..iter]);
    if rust_str_res.is_err() {
        return Err(Errno::EINVAL);
    }

    Ok(String::from(rust_str_res.unwrap()))
}

/// copies the bytes until the lenght of the buffer, then remaining
/// length with 0s.
pub fn copy_pad_buffer(src: &[u8], dest: *mut u8, size: usize) -> Result<(), Errno> {
    unsafe {
        let src_len = src.len();
        if size < src_len {
            return Err(Errno::EFAULT);
        }
        let zero_pad_len = size - src_len;
        // copy bytes of the buffer
        ptr::copy_nonoverlapping(src.as_ptr(), dest, src_len);
        ptr::write_bytes(dest.add(src_len), 0, zero_pad_len);
    }

    Ok(())
}
