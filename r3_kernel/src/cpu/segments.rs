use core::mem;

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum PrevillageLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    #[inline]
    pub fn new(index: u16, ring: PrevillageLevel) -> SegmentSelector {
        SegmentSelector(index << 3 | (ring as u16))
    }
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum SegmentRegister {
    CS,
    SS,
    DS,
    ES,
    FS,
    GS,
}

impl SegmentRegister {
    pub fn set(&self, value: u16) {
        match self {
            SegmentRegister::CS => unsafe {
                asm!(
                    "mov cs, {:x}", in(reg) value,
                    options(nostack, preserves_flags)
                )
            },
            SegmentRegister::DS => unsafe {
                asm!(
                    "mov ds, {:x}", in(reg) value,
                    options(nostack, preserves_flags)
                )
            },
            SegmentRegister::ES => unsafe {
                asm!(
                    "mov es, {:x}", in(reg) value,
                    options(nostack, preserves_flags)
                )
            },
            SegmentRegister::FS => unsafe {
                asm!(
                    "mov fs, {:x}", in(reg) value,
                    options(nostack, preserves_flags)
                )
            },
            SegmentRegister::GS => unsafe {
                asm!(
                    "mov gs, {:x}", in(reg) value,
                    options(nostack, preserves_flags)
                )
            },
            SegmentRegister::SS => unsafe {
                asm!(
                    "mov ss, {:x}", in(reg) value,
                    options(nostack, preserves_flags)
                )
            },
        }
    }

    pub fn get(&self) -> u16 {
        let value: u16;
        match self {
            SegmentRegister::CS => unsafe {
                asm!(
                    "mov {:x}, cs", out(reg) value,
                    options(nomem, nostack, preserves_flags)
                )
            },
            SegmentRegister::DS => unsafe {
                asm!(
                    "mov {:x}, ds", out(reg) value,
                    options(nomem, nostack, preserves_flags)
                )
            },
            SegmentRegister::ES => unsafe {
                asm!(
                    "mov {:x}, es", out(reg) value,
                    options(nomem, nostack, preserves_flags)
                )
            },
            SegmentRegister::FS => unsafe {
                asm!(
                    "mov {:x}, fs", out(reg) value,
                    options(nomem, nostack, preserves_flags)
                )
            },
            SegmentRegister::GS => unsafe {
                asm!(
                    "mov {:x}, gs", out(reg) value,
                    options(nomem, nostack, preserves_flags)
                )
            },
            SegmentRegister::SS => unsafe {
                asm!(
                    "mov {:x}, ss", out(reg) value,
                    options(nomem, nostack, preserves_flags)
                )
            },
        }

        return value;
    }
}

const MAX_GDT_ENTRIES: usize = 8;
const RING_3_DPL_FLAG: u64 = 3 << 45;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct GDTPointer {
    pub size_limit: u16,
    pub base_addr: u64,
}

// These are the default GDT flags that are used
// in linux kernel. For our OS, we use the same thing.
pub enum LinuxKernelSegments {
    KernelCode = 0x00af9b000000ffff,
    KernelData = 0x00cf93000000ffff,
    UserCode = 0x00affb000000ffff,
    UserData = 0x00cff3000000ffff,
}

pub struct GlobalDescritorTable {
    // each GDT entry contains a 64-bit value.
    pub entries: [u64; MAX_GDT_ENTRIES],
    // contains the current index to be filled. [0 - first entry]
    pub filled: usize,
}

impl GlobalDescritorTable {
    pub fn empty() -> GlobalDescritorTable {
        GlobalDescritorTable {
            entries: [0; 8],
            filled: 1,
        }
    }

    pub fn from_slices(buffer: &[u64]) -> Result<GlobalDescritorTable, &'static str> {
        let length = buffer.len();
        if length > MAX_GDT_ENTRIES {
            return Err("Maximum GDT entries exceeded.");
        }

        // enter the values to GTD table:
        let mut entries: [u64; MAX_GDT_ENTRIES] = [0; MAX_GDT_ENTRIES];
        let mut filled = 0;
        for entry in buffer {
            entries[filled] = *entry;
            filled = filled + 1;
        }

        Ok(GlobalDescritorTable { entries, filled })
    }

    pub fn as_pointer(&self) -> GDTPointer {
        GDTPointer {
            base_addr: self.entries.as_ptr() as u64,
            size_limit: (mem::size_of::<u64>() * self.filled - 1) as u16,
        }
    }

    pub fn load_into_cpu(&self) {
        let gdt_pointer = self.as_pointer();
        unsafe {
            asm!(
                "lgdt [{}]", in(reg) &gdt_pointer,
                options(readonly, nostack, preserves_flags)
            )
        }
    }

    #[inline]
    fn get_user_seg_ring(entry: u64) -> PrevillageLevel {
        // check if it is DPL3:
        if entry & RING_3_DPL_FLAG == RING_3_DPL_FLAG {
            return PrevillageLevel::Ring3;
        }

        PrevillageLevel::Ring0
    }

    pub fn set_user_segment(&mut self, entry: u64) -> Result<SegmentSelector, &'static str> {
        if self.filled >= MAX_GDT_ENTRIES {
            return Err("GDT is already full, can't add new entry.");
        }

        // add a new entry:
        self.entries[self.filled] = entry;
        self.filled += 1;

        Ok(SegmentSelector::new(
            self.filled as u16,
            GlobalDescritorTable::get_user_seg_ring(entry),
        ))
    }

    #[inline]
    // asserts the given flag is in ring 3
    pub fn assert_ring_3(entry: u64) {
        assert_eq!(entry & RING_3_DPL_FLAG, RING_3_DPL_FLAG);
    }
}

// init GDT for the base processor:
pub fn init_bp_gdt() {

    log::info!("Initializing GDT for the base processor.");

    // create a GDT with empty segment
    let gdt = GlobalDescritorTable::empty();
    
}
