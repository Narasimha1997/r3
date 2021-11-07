extern crate log;

use crate::boot_proto::BootProtocol;

pub mod paging;
pub mod phy;
pub mod heap;

// some types related to memory management

pub enum MemorySizes {
    OneKiB = 1 * 1024,
    OneMib = 1 * 1024 * 1024,
    OneGiB = 1 * 1024 * 1024 * 1024
}

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
    pub fn get_mut_ptr<T>(self) -> *mut T {
        self.as_u64() as *mut T
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

    #[inline]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        VirtualAddress::from_u64(ptr as u64)
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

pub fn init() {
    log::info!("Enabling frame allocator...");
    phy::setup_physical_memory();
    log::info!("Enabling kernel paging...");
    paging::setup_paging();

    run_initial_paging_test();

    // init kenel heap
    log::info!("Enabling kernel heap...");
    heap::init_heap();
}

#[inline]
pub fn run_initial_paging_test() {
    log::info!("Running simple paging test....");

    // some dummy value:
    let expected_value: u64 = 0x34445544;

    log::debug!("The expected value is at virtual address={:p}", &expected_value);

    let k_table = paging::get_kernel_table();
    let phy_addr = k_table.translate(VirtualAddress::from_ptr(&expected_value));

    if phy_addr.is_none() {
        panic!(
            "Paging test failed. Kernel page table returned null for virtual address: {:p}",
            &expected_value
        );
    }

    // check if the difference between physical address and virtual address == phy_offset
    let phy_offset = BootProtocol::get_phy_offset();
    let v_result_addr = phy_offset.unwrap() + phy_addr.unwrap().as_u64();
    let value: &u64 = unsafe { &*(v_result_addr as *const u64) };

    assert_eq!(expected_value, *value);

    log::info!(
        "Virtual Memory test passed, expected=0x{:x}, got=0x{:x}",
        expected_value,
        value
    );
}
