use crate::mm::PhysicalAddress;

const CR3_PHY_ADDR_MASK: u64 = 0x000ffffffffff000;

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

pub fn get_page_table_flags() -> u16 {
    let cr3_val = read_cr3();
    (cr3_val & 0xfff) as u16
}
