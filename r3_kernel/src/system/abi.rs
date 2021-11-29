extern crate log;

use crate::acpi::lapic::LAPICUtils;

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::SyscallRegsState;

use crate::mm::VirtualAddress;

use crate::system::posix::dispatch_syscall;

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
    ENOSYS = 32,
    EINVAL = 22,
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
