pub mod paging;

// some types related to memory management

pub enum PageTableLevel {
    Level4,
    Level3,
    Level2,
    Level1,
}

pub struct Alignment;

impl Alignment {
    pub fn align_down(addr: u64, size: u64) -> u64 {
        addr & !(size - 1)
    }

    pub fn align_up(addr: u64, size: u64) -> u64 {
        if addr & (size - 1) == 0 {
            addr
        } else {
            addr | size
        }
    }
}

/// Represents a virtual 64-bit address.
#[derive(Debug, Clone, Copy)]
pub struct VirtualAddress(u64);

/// Represents a physical 64-bit address.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalAddress(u64);

impl VirtualAddress {
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn from_u64(addr: u64) -> Self {
        VirtualAddress(addr)
    }

    #[inline]
    pub fn is_aligned_at(&self, size: u64) -> bool {
        self.0 == Alignment::align_down(self.0, size)
    }

    #[inline]
    pub fn align_down(&mut self, size: u64) {
        self.0 = Alignment::align_down(self.0, size);
    }

    #[inline]
    pub fn align_up(&mut self, size: u64) {
        self.0 = Alignment::align_up(self.0, size);
    }

    #[inline]
    pub fn new_align_down(&self, size: u64) -> VirtualAddress {
        VirtualAddress::from_u64(Alignment::align_down(self.0, size))
    }

    #[inline]
    pub fn new_align_up(&self, size: u64) -> VirtualAddress {
        VirtualAddress::from_u64(Alignment::align_up(self.0, size))
    }

    #[inline]
    pub fn get_ptr<T>(self) -> *const T {
        self.as_u64() as *const T
    }

    #[inline]
    pub fn get_level_index(&self, level: PageTableLevel) -> paging::PageTableIndex {
        match level {
            PageTableLevel::Level4 => {
                return paging::PageTableIndex::new((self.0 >> 12 >> 9 >> 9 >> 9) as u16);
            }
            PageTableLevel::Level3 => {
                return paging::PageTableIndex::new((self.0 >> 12 >> 9 >> 9) as u16);
            }
            PageTableLevel::Level2 => {
                return paging::PageTableIndex::new((self.0 >> 12 >> 9) as u16);
            }
            PageTableLevel::Level1 => {
                return paging::PageTableIndex::new((self.0 >> 12) as u16);
            }
        }
    }

    #[inline]
    pub fn get_page_offset(&self) -> u16 {
        self.0 as u16 % (1 << 12)
    }
}

impl PhysicalAddress {
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn from_u64(addr: u64) -> Self {
        PhysicalAddress(addr)
    }

    #[inline]
    pub fn is_aligned_at(&self, size: u64) -> bool {
        self.0 == Alignment::align_down(self.0, size)
    }

    #[inline]
    pub fn align_down(&mut self, size: u64) {
        self.0 = Alignment::align_down(self.0, size);
    }

    #[inline]
    pub fn align_up(&mut self, size: u64) {
        self.0 = Alignment::align_up(self.0, size);
    }

    #[inline]
    pub fn new_align_down(&self, size: u64) -> Self {
        PhysicalAddress::from_u64(Alignment::align_down(self.0, size))
    }

    #[inline]
    pub fn new_align_up(&self, size: u64) -> Self {
        PhysicalAddress::from_u64(Alignment::align_up(self.0, size))
    }
}
