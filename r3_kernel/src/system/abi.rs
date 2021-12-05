extern crate alloc;
extern crate log;

use crate::acpi::lapic::LAPICUtils;

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::SyscallRegsState;

use crate::mm::VirtualAddress;

use crate::system::posix::dispatch_syscall;
use alloc::{string::String, vec};
use core::str;

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
    EEXIST = 17,
    ENOSYS = 32,
    EINVAL = 22,
    EMFILE = 24,
    ENAMETOOLONG = 63,
}

pub type UserAddress = VirtualAddress;

#[no_mangle]
pub extern "sysv64" fn syscall_handler(
    _frame: &mut InterruptStackFrame,
    regs: &mut SyscallRegsState,
) {
    // dispatch handler with arguments collected:
    let sys_no = regs.rax as usize;
    let arg0 = regs.rdi as usize;
    let arg1 = regs.rsi as usize;
    let arg2 = regs.rdx as usize;

    log::info!("Syscall!");
    let result = dispatch_syscall(sys_no, arg0, arg1, arg2);

    regs.rax = result as u64;

    LAPICUtils::eoi();
}

/// copies a c-like string to rust String on kernel heap.
pub fn copy_cstring(source: VirtualAddress, max_len: usize) -> Result<String, Errno> {
    let mut buffer = vec![0; max_len];

    // copy until null terminated
    let c_ptr = source.get_ptr::<u8>();
    let mut iter = 0;

    // good old C
    unsafe {
        while *c_ptr != b'\0' {
            buffer[iter] = *c_ptr;
            iter = iter + 1;

            if iter >= 512 {
                return Err(Errno::ENAMETOOLONG);
            }
        }
    }

    let rust_str_res = str::from_utf8(&buffer[0..iter]);
    if rust_str_res.is_err() {
        return Err(Errno::EINVAL);
    }

    Ok(String::from(rust_str_res.unwrap()))
}
