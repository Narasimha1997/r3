extern crate bit_field;
extern crate bitflags;
extern crate log;

use crate::cpu::mmu;

use crate::boot_proto::BootProtocol;
use crate::mm;
use crate::mm::phy::{Frame, PhysicalMemoryManager};
use lazy_static::lazy_static;

use bit_field::BitField;
use bitflags::bitflags;

const MAX_ENTRIES_PER_LEVEL: u16 = 512;
const ENTRY_ADDR_BIT_MASK: u64 = 0x000ffffffffff000;
const PAGE_TABLE_SIZE: u64 = 0x1000; // 4KB

#[derive(Debug)]
pub enum PagingError {
    OOM,
    UnsupportedFeature,
    OutOfBoundsIndex(u16),
    UnalignedAddress(u64),
    MappingError(u64),
    IsAlreadyMapped(u64),
    PageNotMapped(u64),
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

impl PageEntryFlags {
    #[inline]
    pub fn kernel_flags() -> PageEntryFlags {
        let value: u64 = PageEntryFlags::PRESENT.bits() | PageEntryFlags::READ_WRITE.bits();
        PageEntryFlags::from_bits_truncate(value)
    }

    #[inline]
    pub fn kernel_hugepage_flags() -> PageEntryFlags {
        let value: u64 = PageEntryFlags::PRESENT.bits()
            | PageEntryFlags::READ_WRITE.bits()
            | PageEntryFlags::HUGE_PAGE.bits();
        let flags = PageEntryFlags::from_bits_truncate(value);
        return flags;
    }

    #[inline]
    pub fn user_flags() -> PageEntryFlags {
        let value: u64 = PageEntryFlags::PRESENT.bits()
            | PageEntryFlags::READ_WRITE.bits()
            | PageEntryFlags::USERSPACE.bits();
        let flags = PageEntryFlags::from_bits_truncate(value);
        return flags;
    }

    #[inline]
    pub fn user_hugepage_flags() -> PageEntryFlags {
        let value: u64 = PageEntryFlags::PRESENT.bits()
            | PageEntryFlags::READ_WRITE.bits()
            | PageEntryFlags::HUGE_PAGE.bits()
            | PageEntryFlags::USERSPACE.bits();
        let flags = PageEntryFlags::from_bits_truncate(value);
        return flags;
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

    #[inline]
    pub fn set_usermode_flag(&mut self) {
        self.0 = self.0 | PageEntryFlags::USERSPACE.bits();
    }
}

#[derive(Clone, Debug)]
#[repr(align(4096), C)]
pub struct PageTable {
    pub entries: [PageEntry; MAX_ENTRIES_PER_LEVEL as usize],
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

#[derive(Debug, Clone)]
pub struct VirtualMemoryManager {
    pub n_tables: usize,
    pub l4_virtual_address: mm::VirtualAddress,
    pub l4_phy_addr: mm::PhysicalAddress,
    pub phy_offset: u64,
    pub offset_base_addr: mm::PhysicalAddress,
    pub l4_offset_addr: mm::VirtualAddress,
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

        mmu::reload_flush();

        VirtualMemoryManager {
            n_tables: 4,
            l4_virtual_address: mapped_vmm_addr,
            l4_phy_addr: current_pt_addr,
            phy_offset,
            offset_base_addr: current_pt_addr,
            l4_offset_addr: mapped_vmm_addr,
        }
    }

    #[inline]
    fn get_level_address(&self, next_addr: u64) -> mm::VirtualAddress {
        let offset = next_addr - self.offset_base_addr.as_u64();
        mm::VirtualAddress(self.l4_offset_addr.as_u64() + offset)
    }

    #[inline]
    fn get_or_create_table(
        &self,
        entry: &mut PageEntry,
        create: bool,
    ) -> Option<&'static mut PageTable> {
        if entry.is_mapped() {
            let pt: &mut PageTable =
                unsafe { &mut *self.get_level_address(entry.addr().as_u64()).get_mut_ptr() };
            
            entry.set_flags(PageEntryFlags::user_flags());

            return Some(pt);
        }

        if create {
            let frame_for_pt_opt = PhysicalMemoryManager::alloc();
            if frame_for_pt_opt.is_none() {
                panic!("Failed to create new page table because of OOM.");
            }

            let frame_addr = frame_for_pt_opt.unwrap().addr();

            // set address:
            let res = entry.set_address(frame_addr, PageEntryFlags::user_flags());
            if res.is_err() {
                panic!("{:?}", res.unwrap_err());
            }

            // create the PageTable from frame and reset it:
            let new_pt: &mut PageTable = unsafe {
                &mut *mm::VirtualAddress::from_u64(frame_addr.as_u64() + self.phy_offset)
                    .get_mut_ptr()
            };

            log::debug!(
                "Created new page table at phy=0x{:x} virt={:p}",
                frame_addr.as_u64(),
                &new_pt
            );

            new_pt.reset();
            return Some(new_pt);
        }

        return None;
    }

    #[inline]
    fn walk_hierarchy(
        &self,
        address: &mm::VirtualAddress,
        create: bool,
        assert_huge_page: bool,
        l3: bool,
    ) -> Option<&'static mut PageEntry> {
        let l4_table: &mut PageTable = unsafe { &mut *self.l4_virtual_address.get_mut_ptr() };

        let l4_index = address.get_level_index(mm::PageTableLevel::Level4);
        let l3_index = address.get_level_index(mm::PageTableLevel::Level3);
        let l2_index = address.get_level_index(mm::PageTableLevel::Level2);

        // l3 table
        let l4_entry: &mut PageEntry = &mut l4_table.entries[l4_index.as_usize()];
        let l3_table_opt = self.get_or_create_table(l4_entry, create);
        if l3_table_opt.is_none() {
            log::debug!("l3 not found!");
            return None;
        }
        let l3_table = l3_table_opt.unwrap();

        // l2 table:
        let l3_entry: &mut PageEntry = &mut l3_table.entries[l3_index.as_usize()];
        if l3 {
            return Some(l3_entry);
        }

        let l2_table_opt = self.get_or_create_table(l3_entry, create);
        if l2_table_opt.is_none() {
            return None;
        }

        let l2_table = l2_table_opt.unwrap();

        let l2_entry: &mut PageEntry = &mut l2_table.entries[l2_index.as_usize()];
        if assert_huge_page {
            assert_eq!(l2_entry.has_flag(PageEntryFlags::HUGE_PAGE), true);
        }

        Some(l2_entry)
    }

    pub fn translate_to_frame(&self, address: &mm::VirtualAddress) -> Option<Frame> {
        let resolved_opt = self.walk_hierarchy(address, false, false, false);

        if resolved_opt.is_none() {
            return None;
        }

        let l2_entry = resolved_opt.unwrap();

        if l2_entry.has_flag(PageEntryFlags::HUGE_PAGE) {
            return Frame::from_aligned_address(l2_entry.addr()).ok();
        }

        let l1_index = address.get_level_index(mm::PageTableLevel::Level1);

        let l1_table_opt = self.get_or_create_table(l2_entry, false);
        if l1_table_opt.is_none() {
            log::debug!("l1 not found!");
            return None;
        }

        let l1_table = l1_table_opt.unwrap();

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

    pub fn map_page(
        &self,
        page: Page,
        frame: Frame,
        flags: PageEntryFlags,
    ) -> Result<(), PagingError> {
        let resolved_opt = self.walk_hierarchy(&page.addr(), true, false, false);

        if resolved_opt.is_none() {
            return Err(PagingError::MappingError(page.as_u64()));
        }

        let l2_entry = resolved_opt.unwrap();

        if l2_entry.has_flag(PageEntryFlags::HUGE_PAGE) {
            return Err(PagingError::IsAlreadyMapped(page.as_u64()));
        }

        // create a 4k page from the l2 address:
        let l1_table_opt = self.get_or_create_table(l2_entry, true);
        if l1_table_opt.is_none() {
            return Err(PagingError::MappingError(page.as_u64()));
        }

        let l1_table = l1_table_opt.unwrap();

        let l1_index = page.addr().get_level_index(mm::PageTableLevel::Level1);
        // check if it is already mapped:
        let l1_entry: &PageEntry = &l1_table.entries[l1_index.as_usize()];
        if l1_entry.is_mapped() {
            return Err(PagingError::IsAlreadyMapped(page.as_u64()));
        }

        // not mapped, create a new page:
        let mut page_entry = PageEntry::empty();
        page_entry.set_phy_frame(frame, flags);
        l1_table.entries[l1_index.as_usize()] = page_entry;

        // reload tlb
        mmu::reload_flush();

        Ok(())
    }

    pub fn map_huge_page(
        &self,
        page: Page,
        frame: Frame,
        flags: PageEntryFlags,
    ) -> Result<(), PagingError> {
        let resolved_opt = self.walk_hierarchy(&page.addr(), true, false, true);
        if resolved_opt.is_none() {
            log::debug!("Walk error");
            return Err(PagingError::MappingError(page.as_u64()));
        }

        // create a huge page from that physical address:
        let l3_entry = resolved_opt.unwrap();
        let l2_index = page.addr().get_level_index(mm::PageTableLevel::Level2);

        let l2_table_opt = self.get_or_create_table(l3_entry, true);
        if l2_table_opt.is_none() {
            return Err(PagingError::MappingError(page.addr().as_u64()));
        }

        // map to l2 table:
        let l2_table = l2_table_opt.unwrap();
        let page_entry: &PageEntry = &l2_table.entries[l2_index.as_usize()];

        if page_entry.is_mapped() {
            log::error!("Address already mapped!");
            return Err(PagingError::MappingError(page.addr().as_u64()));
        }

        l2_table.entries[l2_index.as_usize()].set_phy_frame(frame, flags);
        Ok(())
    }

    pub fn map_from_address(
        &self,
        va: mm::VirtualAddress,
        pa: mm::PhysicalAddress,
        flags: PageEntryFlags,
        huge_page: bool,
    ) -> Result<(), PagingError> {
        let align_size = if huge_page {
            PageSize::Page2MiB.size()
        } else {
            PageSize::Page4KiB.size()
        };

        if !va.is_aligned_at(align_size) {
            return Err(PagingError::UnalignedAddress(va.as_u64()));
        }

        if !pa.is_aligned_at(PageSize::Page4KiB.size()) {
            return Err(PagingError::UnalignedAddress(pa.as_u64()));
        }

        // create Page and Frame
        let page = Page::from_address(va);
        let frame = Frame::from_address(pa);

        if huge_page {
            return self.map_huge_page(page, frame, flags);
        }

        self.map_page(page, frame, flags)
    }

    fn unmap_single(&self, page: Page) -> Result<(), PagingError> {
        let resolved_opt = self.walk_hierarchy(&page.addr(), false, false, false);
        if resolved_opt.is_none() {
            return Err(PagingError::PageNotMapped(page.as_u64()));
        }

        // get the address and huge page flag:
        let l2_entry = resolved_opt.unwrap();

        if !l2_entry.is_mapped() {
            return Err(PagingError::PageNotMapped(page.as_u64()));
        }

        if l2_entry.has_flag(PageEntryFlags::HUGE_PAGE) {
            l2_entry.unmap_entry();
            return Ok(());
        }

        // reset the region to zero:
        let l1_index = page.addr().get_level_index(mm::PageTableLevel::Level1);
        let l1_table_opt = self.get_or_create_table(l2_entry, false);
        if l1_table_opt.is_none() {
            return Err(PagingError::PageNotMapped(page.as_u64()));
        }
        let l1_table = l1_table_opt.unwrap();

        let l1_entry: &PageEntry = &l1_table.entries[l1_index.as_usize()];
        if !l1_entry.is_mapped() {
            return Err(PagingError::PageNotMapped(page.as_u64()));
        }

        // unmap the page
        l1_table.entries[l1_index.as_usize()].unmap_entry();
        return Ok(());
    }

    pub fn unmap_page(&self, page: Page) -> Result<(), PagingError> {
        let result = self.unmap_single(page);
        if result.is_err() {
            return result;
        }

        mmu::reload_flush();
        return result;
    }
}

#[derive(Clone, Debug)]
pub struct PageRange {
    pub start: mm::VirtualAddress,
    pub n: usize,
    pub size: PageSize,
}

impl PageRange {
    pub fn new(start: mm::VirtualAddress, n: usize, size: PageSize) -> Self {
        PageRange { start, n, size }
    }
}

#[derive(Debug)]
pub struct PageRangeIterator {
    pub page_range: PageRange,
    pub current: usize,
}

impl PageRangeIterator {
    pub fn new(page_range: PageRange) -> Self {
        PageRangeIterator {
            page_range,
            current: 0,
        }
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }
}

impl Iterator for PageRangeIterator {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.page_range.n {
            return None;
        }

        let current_page = Page::from_address(mm::VirtualAddress::from_u64(
            self.page_range.start.as_u64() + self.current as u64 * self.page_range.size.size(),
        ));

        self.current += 1;

        Some(current_page)
    }
}

pub fn init_kernel_vmm() -> VirtualMemoryManager {
    let phy_offset = BootProtocol::get_phy_offset();
    if phy_offset.is_none() {
        panic!("Boot protocol did not provide physical memory offset.");
    }

    VirtualMemoryManager::from_cr3(phy_offset.unwrap())
}

lazy_static! {
    pub static ref KERNEL_PAGING: VirtualMemoryManager = init_kernel_vmm();
}

pub fn setup_paging() {
    // this function will make static lazy function to initialize
    log::info!(
        "Kernel paging is initialized, address at: 0x{:x}",
        KERNEL_PAGING.l4_virtual_address.as_u64()
    );
}

pub fn get_kernel_table() -> &'static VirtualMemoryManager {
    &KERNEL_PAGING
}

/// provides simple virtual memory allocation functions over virtual
/// memory page table of the kernel
pub struct KernelVirtualMemoryManager;

impl KernelVirtualMemoryManager {
    pub fn pt() -> &'static VirtualMemoryManager {
        &KERNEL_PAGING
    }

    pub fn alloc_page(
        address: mm::VirtualAddress,
        flags: PageEntryFlags,
    ) -> Result<Page, PagingError> {
        // allocate a physical frame
        let frame = PhysicalMemoryManager::alloc();
        if frame.is_none() {
            return Err(PagingError::OOM);
        }

        // allocate the page
        let result = KERNEL_PAGING.map_page(Page::from_address(address), frame.unwrap(), flags);
        if result.is_err() {
            return Err(result.unwrap_err());
        }

        return Ok(Page::from_address(address));
    }

    pub fn alloc_huge_page(
        address: mm::VirtualAddress,
        flags: PageEntryFlags,
    ) -> Result<Page, PagingError> {
        let alloc_opt = PhysicalMemoryManager::alloc_huge_page();
        if alloc_opt.is_none() {
            return Err(PagingError::OOM);
        }

        // allocate frame
        let frame = alloc_opt.unwrap();

        let result = KERNEL_PAGING.map_huge_page(Page::from_address(address), frame, flags);
        if result.is_err() {
            return Err(result.unwrap_err());
        }

        return Ok(Page::from_address(address));
    }

    pub fn alloc_region(
        region: PageRange,
        flags: PageEntryFlags,
    ) -> Result<PageRangeIterator, PagingError> {
        let range_iterator = PageRangeIterator::new(region.clone());

        for page in range_iterator {
            let frame_opt = PhysicalMemoryManager::alloc();
            if frame_opt.is_none() {
                return Err(PagingError::OOM);
            }

            // map the page
            let result = KERNEL_PAGING.map_page(page, frame_opt.unwrap(), flags);
            if result.is_err() {
                return Err(result.unwrap_err());
            }
        }

        // return the iterator
        Ok(PageRangeIterator::new(region))
    }

    pub fn alloc_huge_page_region(
        region: PageRange,
        flags: PageEntryFlags,
    ) -> Result<PageRangeIterator, PagingError> {
        let range_iterator = PageRangeIterator::new(region.clone());

        for page in range_iterator {
            let frame_opt = PhysicalMemoryManager::alloc_huge_page();

            if frame_opt.is_none() {
                return Err(PagingError::OOM);
            }

            // map the page:
            let result = KERNEL_PAGING.map_huge_page(page, frame_opt.unwrap(), flags);
            if result.is_err() {
                return Err(result.unwrap_err());
            }
        }

        mmu::reload_flush();

        Ok(PageRangeIterator::new(region))
    }

    pub fn free_page(address: mm::VirtualAddress) -> Result<(), PagingError> {
        KERNEL_PAGING.unmap_page(Page::from_address(address))
    }

    pub fn free_region(region: PageRange) -> Result<(), PagingError> {
        for page in PageRangeIterator::new(region) {
            let result = KERNEL_PAGING.unmap_single(page);
            if result.is_err() {
                return result;
            }
        }

        // flush tlb:
        mmu::reload_flush();
        Ok(())
    }

    pub fn new_vmm() -> (VirtualMemoryManager, mm::PhysicalAddress) {
        let k_vmm = KernelVirtualMemoryManager::pt();
        // allocate a new virtual address at 4k aligned region for new virtual address:
        let frame_opt = PhysicalMemoryManager::alloc();
        if frame_opt.is_none() {
            panic!("Failed to allocate memory for new Virtual page table. OOM");
        }

        let frame = frame_opt.unwrap();

        // get it's address:
        let new_pt_vaddr = mm::VirtualAddress::from_u64(k_vmm.phy_offset + frame.as_u64());

        // clone the page table
        let page_table: &mut PageTable = unsafe { &mut *new_pt_vaddr.get_mut_ptr() };

        // copy the pages of kernel p4 table:
        let kernel_table: &mut PageTable = unsafe { &mut *k_vmm.l4_virtual_address.get_mut_ptr() };

        for idx in 256..kernel_table.entries.len() {
            page_table.entries[idx] = kernel_table.entries[idx].clone();
            page_table.entries[idx].set_usermode_flag();
        }

        (
            VirtualMemoryManager {
                n_tables: 1,
                l4_virtual_address: new_pt_vaddr,
                l4_phy_addr: frame.addr(),
                phy_offset: k_vmm.phy_offset,
                offset_base_addr: k_vmm.l4_phy_addr,
                l4_offset_addr: k_vmm.l4_virtual_address,
            },
            frame.addr(),
        )
    }
}
