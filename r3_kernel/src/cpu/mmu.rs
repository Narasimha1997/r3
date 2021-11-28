extern crate bitflags;

use crate::mm::PhysicalAddress;
use bitflags::bitflags;

const CR3_PHY_ADDR_MASK: u64 = 0x000ffffffffff000;

bitflags! {
    #[repr(transparent)]
    pub struct PageFaultExceptionTypes: u64 {
        const PROTECTION_VIOLATION = 1;
        const CAUSED_BY_WRITE = 1 << 1;
        const USER_MODE = 1 << 2;
        const MALFORMED_TABLE = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 4;
        const PROTECTION_KEY = 1 << 5;
        const SHADOW_STACK = 1 << 6;
        const SGX = 1 << 15;
        const RMP = 1 << 31;
    }
}

pub fn read_cr3() -> u64 {
    let cr3_val: u64;
    unsafe {
        asm!(
            "mov {}, cr3", out(reg) cr3_val,
            options(nomem, nostack, preserves_flags)
        );
    }

    cr3_val
}

pub fn get_page_table_address() -> PhysicalAddress {
    let cr3_val = read_cr3();
    PhysicalAddress::from_u64(cr3_val & CR3_PHY_ADDR_MASK)
}

pub fn set_page_table_address(addr: PhysicalAddress) {
    let masked_value = addr.as_u64() & CR3_PHY_ADDR_MASK;
    write_cr3(masked_value);   
}

pub fn get_page_table_flags() -> u16 {
    let cr3_val = read_cr3();
    (cr3_val & 0xfff) as u16
}

pub fn write_cr3(value: u64) {
    unsafe {
        asm!(
            "mov cr3, {}",
            in(reg) value,
            options(nostack, preserves_flags)
        );
    }
}

pub fn read_cr2() -> u64 {
    let cr2_val: u64;
    unsafe {
        asm!(
            "mov {}, cr2",
            out(reg) cr2_val,
            options(nostack, nomem, preserves_flags)
        )
    }

    cr2_val
}

pub fn reload_flush() {
    // reloading the CR3 register will cause TLB to flush automatically.
    let cr3_val = read_cr3();
    write_cr3(cr3_val);
}
