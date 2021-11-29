extern crate log;

use crate::cpu::interrupts::InterruptStackFrame;
use crate::cpu::state::SyscallRegsState;
use crate::acpi::lapic::LAPICUtils;

#[no_mangle]
pub extern "sysv64" fn syscall_handler(
    _frame: &mut InterruptStackFrame,
    regs: &mut SyscallRegsState,
) {
    // don't do anything
    log::info!("Got syscall!!");
    regs.rax = 1;

    LAPICUtils::eoi();
}
