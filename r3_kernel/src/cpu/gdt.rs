use core::mem;

const MAX_GDT_ENTRIES: usize = 8;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct GDTPointer {
    pub size_limit: u16,
    pub base_addr: u64,
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
        let filled = if self.filled == 0 { 1 } else { self.filled };

        GDTPointer {
            base_addr: self.entries.as_ptr() as u64,
            size_limit: (mem::size_of::<u64>() * filled - 1) as u16,
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
}
