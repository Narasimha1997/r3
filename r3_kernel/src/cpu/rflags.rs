extern crate bitflags;

use bitflags::bitflags;

bitflags! {
    /// The RFLAGS register.
    pub struct RFlagsStruct: u64 {
        const ID = 1 << 21;
        const VIRTUAL_INTERRUPT_PENDING = 1 << 20;
        const VIRTUAL_INTERRUPT = 1 << 19;
        const ALIGNMENT_CHECK = 1 << 18;
        const VIRTUAL_8086_MODE = 1 << 17;
        const RESUME_FLAG = 1 << 16;
        const NESTED_TASK = 1 << 14;
        const IOPL_HIGH = 1 << 13;
        const IOPL_LOW = 1 << 12;
        const OVERFLOW_FLAG = 1 << 11;
        const DIRECTION_FLAG = 1 << 10;
        const INTERRUPT_FLAG = 1 << 9;
        const TRAP_FLAG = 1 << 8;
        const SIGN_FLAG = 1 << 7;
        const ZERO_FLAG = 1 << 6;
        const AUXILIARY_CARRY_FLAG = 1 << 4;
        const PARITY_FLAG = 1 << 2;
        const CARRY_FLAG = 1;
    }
}

pub struct RFlags {}

impl RFlags {
    pub fn read() -> u64 {
        let rflags: u64;
        unsafe {
            asm!(
                "pushfq; pop {}", out(reg) rflags,
                options(nomem, preserves_flags)
            );
        }

        rflags
    }

    pub fn read_structured() -> RFlagsStruct {
        let raw = RFlags::read();
        RFlagsStruct::from_bits_truncate(raw)
    }

    pub fn write_structured(flags: RFlagsStruct) {
        let old = RFlags::read();
        RFlags::write((old & !(RFlagsStruct::all().bits())) | flags.bits());
    }

    pub fn write(value: u64) {
        unsafe {
            asm!(
                "push {}; popfq", in(reg) value,
                options(nomem, preserves_flags)
            );
        }
    }

    pub fn is_set(flag: RFlagsStruct) -> bool {
        let r_flags = RFlags::read_structured();
        r_flags.contains(flag)
    }
}
