extern crate bit_field;
extern crate bitflags;

use crate::mm;
use bit_field::BitField;
use bitflags::bitflags;

const MAX_ENTRIES_PER_LEVEL: u16 = 512;

pub enum PagingError {
    OutOfBoundsIndex(u16),
    UnalignedAddress(mm::VirtualAddress),
}

#[derive(Debug, Clone)]
pub enum PageSize {
    Page4KiB,
    Page2MiB,
    Page1GiB,
}

impl PageSize {
    #[inline]
    pub fn size(&self) -> u64 {
        match self {
            Self::Page4KiB => 4 * 1024,
            Self::Page2MiB => 2 * 1024 * 1024,
            Self::Page1GiB => 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    pub fn new(value: u16) -> Self {
        PageTableIndex(value % MAX_ENTRIES_PER_LEVEL)
    }

    #[inline]
    pub fn new_safe(value: u16) -> Result<Self, PagingError> {
        if value >= MAX_ENTRIES_PER_LEVEL {
            return Err(PagingError::OutOfBoundsIndex(value));
        }

        return Ok(PageTableIndex(value % MAX_ENTRIES_PER_LEVEL));
    }

    #[inline]
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
/// Represents a 4KiB Page
pub struct Page(mm::VirtualAddress);

impl Page {
    pub fn from_aligned_address(addr: mm::VirtualAddress) -> Result<Self, PagingError> {
        if !addr.is_aligned_at(PageSize::Page4KiB.size()) {
            return Err(PagingError::UnalignedAddress(addr));
        }

        Ok(Page(addr))
    }

    pub fn from_address(addr: mm::VirtualAddress) -> Self {
        Page(addr.new_align_down(PageSize::Page4KiB.size()))
    }

    #[inline]
    pub fn addr(&self) -> mm::VirtualAddress {
        self.0
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }

    pub fn from_l3_index(p4: PageTableIndex, p3: PageTableIndex) -> Self {
        let mut va = 0;
        va.set_bits(39..48, u64::from(p4.as_u16()));
        va.set_bits(30..39, u64::from(p3.as_u16()));

        Page::from_address(mm::VirtualAddress::from_u64(va))
    }

    pub fn from_l2_index(p4: PageTableIndex, p3: PageTableIndex, p2: PageTableIndex) -> Self {
        let mut va = 0;
        va.set_bits(39..48, u64::from(p4.as_u16()));
        va.set_bits(30..39, u64::from(p3.as_u16()));
        va.set_bits(21..30, u64::from(p2.as_u16()));

        Page::from_address(mm::VirtualAddress::from_u64(va))
    }

    pub fn from_l1_index(
        p4: PageTableIndex,
        p3: PageTableIndex,
        p2: PageTableIndex,
        p1: PageTableIndex,
    ) -> Self {
        let mut va = 0;
        va.set_bits(39..48, u64::from(p4.as_u16()));
        va.set_bits(30..39, u64::from(p3.as_u16()));
        va.set_bits(21..30, u64::from(p2.as_u16()));
        va.set_bits(12..21, u64::from(p1.as_u16()));

        Page::from_address(mm::VirtualAddress::from_u64(va))
    }
}

bitflags! {
    pub struct PageEntryFlags: u64 {
        const PRESENT = 1;
        const READ_WRITE = 1 << 1;
        const USERSPACE = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const NO_CACHE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const RW_ONLY = 1 << 63;
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct PageEntry(u64);

impl PageEntry {
    #[inline]
    pub fn empty() -> Self {
        PageEntry(0)
    }

    #[inline]
    pub fn is_mapped(&self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn unmap_entry(&mut self) {
        self.0 = 0;
    }
}
