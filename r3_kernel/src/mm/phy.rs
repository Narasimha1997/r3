extern crate bootloader;
extern crate log;

use core::iter::Iterator;

use crate::boot_proto::BootProtocol;
use crate::mm;
use crate::mm::paging::{PageSize, PagingError};
use bootloader::boot_info::{MemoryRegionKind, MemoryRegions};

use lazy_static::lazy_static;

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

/*
    This concept is inspired from:
    https://github.com/phil-opp/blog_os/blob/post-12/src/memory.rs
    The original credits goes to the author of blog_os.
*/

pub trait PhyFrameAllocator {
    /// allocate a single frame
    fn frame_alloc(&mut self) -> Option<Frame>;

    /// deallocate a frame
    fn frame_dealloc(&mut self, index: usize);
}

pub struct LinearFrameAllocator {
    pub memory_regions: &'static MemoryRegions,
    pub next_index: usize,
}

impl LinearFrameAllocator {
    pub fn init() -> Self {
        let memory_map_opt = BootProtocol::get_memory_regions();
        if memory_map_opt.is_none() {
            panic!("Bootloader did not provide memory map.");
        }

        LinearFrameAllocator {
            memory_regions: memory_map_opt.unwrap(),
            next_index: 0,
        }
    }

    #[inline]
    fn create_iterator(&self) -> impl Iterator<Item = Frame> {
        let region_iterator = self.memory_regions.iter();
        let usable_regions_iter =
            region_iterator.filter(|region| region.kind == MemoryRegionKind::Usable);

        let address_range_iter = usable_regions_iter.map(|region| region.start..region.end);

        let frame_aligned_addresses =
            address_range_iter.flat_map(|addr| addr.step_by(PageSize::Page4KiB.size() as usize));

        // convert into Iterator<Frame> type:
        let frame_iterator = frame_aligned_addresses
            .map(|frame_addr| Frame::from_address(mm::PhysicalAddress::from_u64(frame_addr)));

        frame_iterator
    }
}

impl PhyFrameAllocator for LinearFrameAllocator {
    fn frame_alloc(&mut self) -> Option<Frame> {
        let mut frame_iterator = self.create_iterator();
        let phy_frame = frame_iterator.nth(self.next_index);

        self.next_index += 1;

        phy_frame
    }

    fn frame_dealloc(&mut self, index: usize) {
        log::warn!("Got index={}, Frame deallocation not implemented", index);
    }
}

lazy_static! {
    pub static ref LINEAR_ALLOCATOR: LinearFrameAllocator = LinearFrameAllocator::init();
}

/// a function that lazy initializes LIEAR_ALLOCATOR
pub fn setup_physical_memory() {
    log::info!(
        "Set-up Linear memory allocator for Physical memory successfull, initial_size={}",
        LINEAR_ALLOCATOR.next_index
    );
}

pub struct PhysicalMemoryManager;

impl PhysicalMemoryManager {
    pub fn alloc() -> Option<Frame> {
        LINEAR_ALLOCATOR.frame_alloc()
    }

    pub fn free(frame: Frame) {
        LINEAR_ALLOCATOR.frame_dealloc(0);
    }
}
