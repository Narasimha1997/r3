use lazy_static::lazy_static;

use core::mem;

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum PrevillageLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

// core assembly functions:

// Loads the GDT, after this, the segment register must be reloaded.
fn lgdt(ptr: &GDTPointer) {
    unsafe {
        asm!(
            "lgdt [{0}]", in(reg) ptr,
            options(readonly, nostack, preserves_flags)
        )
    }
}

// CS register cannot be reloaded with the new value like
// other DS, ES, SS, FS or GS registeres. So this is a special case.
fn special_set_cs(value: u16) {
    unsafe {
        asm!(
            "push {sel}",
            "lea {tmp}, [1f + rip]",
            "push {tmp}",
            "retfq",
            "1:",
            sel = in(reg) u64::from(value),
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
    }
}

#[derive(Debug)]
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
            SegmentRegister::CS => special_set_cs(value),
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

    pub fn assert_reg(&self, value: u16) {
        let read_value = self.get();
        assert_eq!(read_value, value);
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
            size_limit: (self.filled * mem::size_of::<u64>() - 1) as u16,
        }
    }

    pub fn load_into_cpu(&'static self) {
        let gdt_pointer = self.as_pointer();
        lgdt(&gdt_pointer);
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
        let current_index = self.filled;
        self.filled += 1;

        let ring = GlobalDescritorTable::get_user_seg_ring(entry);
        Ok(SegmentSelector::new(current_index as u16, ring))
    }

    #[inline]
    // asserts the given flag is in ring 3
    pub fn assert_ring_3(entry: u64) {
        assert_eq!(entry & RING_3_DPL_FLAG, RING_3_DPL_FLAG);
    }
}

pub struct GDTContainer {
    gdt_table: GlobalDescritorTable,
    kernel_code_selector: SegmentSelector,
}

// create GDT for the base processor:
pub fn create_for_bp() -> GDTContainer {
    // create a GDT with empty segment
    let mut gdt = GlobalDescritorTable::empty();
    let k_code_segment_res = gdt.set_user_segment(LinuxKernelSegments::KernelCode as u64);
    if k_code_segment_res.is_err() {
        panic!("{}", k_code_segment_res.unwrap_err());
    }

    GDTContainer {
        gdt_table: gdt,
        kernel_code_selector: k_code_segment_res.unwrap(),
    }
}

lazy_static! {
    static ref KERNEL_BASE_GDT: GDTContainer = create_for_bp();
}

// create the GDT
pub fn init() {
    let gdt_table = &KERNEL_BASE_GDT.gdt_table;
    gdt_table.load_into_cpu();

    // set the code segment register
    let kernel_cs = &KERNEL_BASE_GDT.kernel_code_selector;
    SegmentRegister::CS.set(kernel_cs.0);

    // assert the register value:
    SegmentRegister::CS.assert_reg(kernel_cs.0);
    log::debug!("Verified Code Segment Register value: 0x{:x}", kernel_cs.0);
    log::info!("Initialized GDT.");
}
