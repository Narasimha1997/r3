use crate::cpu::rflags;

use rflags::{RFlags, RFlagsStruct};

pub fn enable() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

pub fn disable() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

pub fn are_enabled() -> bool {
    RFlags::is_set(RFlagsStruct::INTERRUPT_FLAG)
}

pub fn create_breakpoint() {
    unsafe {
        asm!("int3", options(nomem, nostack));
    }
}

pub fn halt_with_interrupts() {
    enable();
    unsafe {
        asm!("hlt", options(nomem, nostack));
    }
}
