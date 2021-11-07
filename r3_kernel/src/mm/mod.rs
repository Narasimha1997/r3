extern crate log;

use crate::boot_proto::BootProtocol;

pub mod paging;
pub mod phy;

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
    run_page_mapping_test();
}

#[inline]
pub fn run_initial_paging_test() {
    log::info!("Running simple paging test....");

    // some dummy value:
    let expected_value: u64 = 0x34445544;

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

#[inline]
fn run_page_mapping_test() {
    // allocate a new page:
    let test_var: u64 = 0x2823324;
    let to_alloc_addr =
        VirtualAddress::from_u64(VirtualAddress::from_ptr(&test_var).as_u64() + (12 * 1024));
    // allocate a frame for this addess:
    let page_res = paging::KernelVirtualMemoryManager::alloc_page(
        to_alloc_addr,
        paging::PageEntryFlags::kernel_flags(),
    );

    if page_res.is_err() {
        panic!("Page map test failed: {:?}", page_res.unwrap_err());
    }

    let address: &mut u64 = unsafe { &mut *page_res.unwrap().addr().get_mut_ptr() };
    *address = test_var;

    // assert value from translation:
    let phy_res = paging::get_kernel_table().translate(VirtualAddress::from_ptr(address));

    if phy_res.is_none() {
        panic!(
            "Paging translation returned null for address={:p}",
            &address
        );
    }

    let phy_addr = phy_res.unwrap().as_u64();
    log::info!("Physical address=0x{:x}", phy_addr);
    let phy_offset = BootProtocol::get_phy_offset().unwrap();

    let formed_va_address = VirtualAddress::from_u64(phy_addr + phy_offset);

    let got_value: &mut u64 = unsafe { &mut *formed_va_address.get_mut_ptr() };

    assert_eq!(test_var, *got_value);

    log::debug!("Expected value: {}, Got: {}", test_var, *got_value);

    *got_value = 0;

    // unmap the page:
    let unmap_res =
        paging::KernelVirtualMemoryManager::free_page(VirtualAddress::from_ptr(got_value));
    if unmap_res.is_err() {
        panic!("Failed to unmap address=0x{:x}", formed_va_address.as_u64());
    }

    log::info!("Paging test passed.");
}
