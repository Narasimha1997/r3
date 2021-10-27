use crate::cpu::rflags;
use crate::cpu::segments;
extern crate bit_field;

use bit_field::BitField;
use rflags::{RFlags, RFlagsStruct};
use segments::SegmentRegister;

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

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptDescriptorEntry {
    pointer_low: u16,
    gdt_selector: u16,
    options: u16,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

const DEFAULT_INTERRUPT_OPTION_BITS: u16 = 0b1110_0000_0000;

impl InterruptDescriptorEntry {
    #[inline]
    pub fn empty() -> Self {
        InterruptDescriptorEntry {
            pointer_low: 0,
            pointer_high: 0,
            pointer_middle: 0,
            options: DEFAULT_INTERRUPT_OPTION_BITS,
            gdt_selector: 0,
            reserved: 0,
        }
    }

    #[inline]
    fn read_cs(&self) -> u16 {
        SegmentRegister::CS.get()
    }

    #[inline]
    fn set_pointers(&mut self, addr: u64) {
        self.pointer_low = (addr & 0xffff) as u16;
        self.pointer_middle = ((addr >> 16) & 0xffff) as u16;
        self.pointer_high = ((addr >> 32) & 0xffffffff) as u32;
    }

    #[inline]
    pub fn get_handler_addr(&self) -> u64 {
        let low = self.pointer_low as u64;
        let middle = (self.pointer_middle as u64) << 16;
        let high = (self.pointer_high as u64) << 32;

        low | high | middle
    }

    #[inline]
    pub fn set_handler(&mut self, handler_address: u64) {
        // set high, low and middle pointers
        self.set_pointers(handler_address);

        // get the cs register:
        self.gdt_selector = self.read_cs();
        self.options.set_bit(15, true);
    }
}
