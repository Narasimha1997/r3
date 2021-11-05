extern crate bit_field;
extern crate bitflags;
extern crate log;

use crate::cpu::mmu;

use crate::boot_proto::BootProtocol;
use crate::mm;
use lazy_static::lazy_static;

use bit_field::BitField;
use bitflags::bitflags;

const MAX_ENTRIES_PER_LEVEL: u16 = 512;
const ENTRY_ADDR_BIT_MASK: u64 = 0x000ffffffffff000;
const PAGE_TABLE_SIZE: u64 = 0x1000; // 4KB

pub enum PagingError {
    OutOfBoundsIndex(u16),
    UnalignedAddress(u64),
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

    #[inline]
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
/// Represents a 4KiB Page
pub struct Page(mm::VirtualAddress);

impl Page {
    pub fn from_aligned_address(addr: mm::VirtualAddress) -> Result<Self, PagingError> {
        if !addr.is_aligned_at(PageSize::Page4KiB.size()) {
            return Err(PagingError::UnalignedAddress(addr.as_u64()));
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

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Frame(mm::PhysicalAddress);

impl Frame {
    pub fn from_aligned_address(addr: mm::PhysicalAddress) -> Result<Self, PagingError> {
        if !addr.is_aligned_at(PageSize::Page4KiB.size()) {
            return Err(PagingError::UnalignedAddress(addr.as_u64()));
        }

        Ok(Frame(addr))
    }

    pub fn from_address(addr: mm::PhysicalAddress) -> Self {
        Frame(addr.new_align_down(PageSize::Page4KiB.size()))
    }

    #[inline]
    pub fn addr(&self) -> mm::PhysicalAddress {
        self.0
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0.as_u64()
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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageEntry(u64);

impl PageEntry {
    #[inline]
    pub fn empty() -> Self {
        PageEntry(0)
    }

    #[inline]
    pub fn empty_from_flags(flags: PageEntryFlags) -> Self {
        PageEntry(0 | flags.bits())
    }

    #[inline]
    pub fn is_mapped(&self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn addr(&self) -> mm::PhysicalAddress {
        mm::PhysicalAddress::from_u64(self.0 & ENTRY_ADDR_BIT_MASK)
    }

    #[inline]
    pub fn unmap_entry(&mut self) {
        self.0 = 0;
    }

    #[inline]
    pub fn set_address(
        &mut self,
        addr: mm::PhysicalAddress,
        flags: PageEntryFlags,
    ) -> Result<(), PagingError> {
        if !addr.is_aligned_at(PageSize::Page4KiB.size()) {
            return Err(PagingError::UnalignedAddress(addr.as_u64()));
        }

        let entry_value = addr.as_u64() | flags.bits();
        self.0 = entry_value;
        Ok(())
    }

    #[inline]
    pub fn set_phy_frame(&mut self, addr: Frame, flags: PageEntryFlags) {
        let phy_addr = addr.as_u64();
        self.0 = phy_addr | flags.bits();
    }

    #[inline]
    pub fn set_flags(&mut self, flags: PageEntryFlags) {
        self.0 = self.addr().as_u64() | flags.bits()
    }

    #[inline]
    pub fn has_flag(&self, flag: PageEntryFlags) -> bool {
        PageEntryFlags::from_bits_truncate(self.0).contains(flag)
    }
}

#[derive(Clone)]
#[repr(align(4096), C)]
pub struct PageTable {
    entries: [PageEntry; MAX_ENTRIES_PER_LEVEL as usize],
}

impl PageTable {
    pub fn empty() -> Self {
        let empty_entry = PageEntry::empty();
        PageTable {
            entries: [empty_entry; MAX_ENTRIES_PER_LEVEL as usize],
        }
    }

    pub fn reset(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.unmap_entry();
        }
    }
}

pub struct VirtualMemoryManager {
    pub n_tables: usize,
    pub l4_virtual_address: mm::VirtualAddress,
    pub l4_phy_addr: mm::PhysicalAddress,
}

impl VirtualMemoryManager {
    #[inline]
    pub fn get_table_addr_by_offset(addr: u64, index: u64) -> u64 {
        addr + index * PAGE_TABLE_SIZE
    }

    pub fn from_cr3(phy_offset: u64) -> VirtualMemoryManager {
        let current_pt_addr = mmu::get_page_table_address();
        assert_eq!(current_pt_addr.is_aligned_at(PAGE_TABLE_SIZE), true);

        // add the physical offset to that address:
        let mapped_vmm_addr = mm::VirtualAddress::from_u64(current_pt_addr.as_u64() + phy_offset);

        log::info!(
            "Page table at Virtual address: 0x{:x}",
            mapped_vmm_addr.as_u64()
        );

        VirtualMemoryManager {
            n_tables: 4,
            l4_virtual_address: mapped_vmm_addr,
            l4_phy_addr: current_pt_addr,
        }
    }

    #[inline]
    fn get_level_address(&self, next_addr: u64) -> mm::VirtualAddress {
        let offset = next_addr - self.l4_phy_addr.as_u64();
        mm::VirtualAddress(self.l4_virtual_address.as_u64() + offset)
    }

    pub fn translate_to_frame(&self, address: &mm::VirtualAddress) -> Option<Frame> {
        let l4_table: &PageTable = unsafe { &*self.l4_virtual_address.get_ptr() };

        let l4_index = address.get_level_index(mm::PageTableLevel::Level4);
        let l3_index = address.get_level_index(mm::PageTableLevel::Level3);
        let l2_index = address.get_level_index(mm::PageTableLevel::Level2);

        let l4_entry: &PageEntry = &l4_table.entries[l4_index.as_usize()];
        if !l4_entry.is_mapped() {
            return None;
        }

        let l3_table: &PageTable =
            unsafe { &*self.get_level_address(l4_entry.addr().as_u64()).get_ptr() };
        let l3_entry: &PageEntry = &l3_table.entries[l3_index.as_usize()];
        if !l3_entry.is_mapped() {
            return None;
        }

        let l2_table: &PageTable =
            unsafe { &*self.get_level_address(l3_entry.addr().as_u64()).get_ptr() };

        // check if it is a 2MiB huge page or it does not exist:
        let l2_entry: &PageEntry = &l2_table.entries[l2_index.as_usize()];
        if !l2_entry.is_mapped() {
            return None;
        }

        // check if it is a huge-page
        if l2_entry.has_flag(PageEntryFlags::HUGE_PAGE) {
            let frame_res = Frame::from_aligned_address(l2_entry.addr());

            return frame_res.ok();
        }

        let l1_index = address.get_level_index(mm::PageTableLevel::Level1);

        let l1_table: &PageTable =
            unsafe { &*self.get_level_address(l2_entry.addr().as_u64()).get_ptr() };
        let l1_entry: &PageEntry = &l1_table.entries[l1_index.as_usize()];
        if !l1_entry.is_mapped() {
            return None;
        }

        Frame::from_aligned_address(l1_entry.addr()).ok()
    }

    pub fn translate(&self, addr: mm::VirtualAddress) -> Option<mm::PhysicalAddress> {
        let translated_frame = self.translate_to_frame(&addr);
        if translated_frame.is_none() {
            return None;
        }

        let phy_u64_frame_addr = translated_frame.unwrap().as_u64();
        let phy_offset = addr.get_page_offset() as u64;

        Some(mm::PhysicalAddress::from_u64(
            phy_u64_frame_addr + phy_offset,
        ))
    }
}

pub struct KernelVirtualMemoryManager {
    pub vmm: VirtualMemoryManager,
    pub phy_offset: mm::PhysicalAddress,
}

pub fn init_kernel_vmm() -> VirtualMemoryManager {
    let phy_offset = BootProtocol::get_phy_offset();
    if phy_offset.is_none() {
        panic!("Boot protocol did not provide physical memory offset.");
    }

    VirtualMemoryManager::from_cr3(phy_offset.unwrap())
}

lazy_static! {
    pub static ref KERNEL_VMM: VirtualMemoryManager = init_kernel_vmm();
}

pub fn setup_paging() {
    // this function will make static lazy function to initialize
    log::info!(
        "Kernel paging is initialized, address at: 0x{:x}",
        KERNEL_VMM.l4_virtual_address.as_u64()
    );
}

pub fn get_kernel_table() -> &'static VirtualMemoryManager {
    &KERNEL_VMM
}
