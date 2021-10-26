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
