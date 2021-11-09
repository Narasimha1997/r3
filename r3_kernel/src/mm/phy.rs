extern crate bootloader;
extern crate log;
extern crate spin;

use crate::boot_proto::BootProtocol;
use crate::mm;
use crate::mm::paging::{PageSize, PagingError};
use bootloader::boot_info::MemoryRegionKind;

use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Frame(mm::PhysicalAddress);

const MAX_FREE_REGIONS: usize = 64;

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

    /// allocate 2MiB amount of Frames i.e 2MiB / 4KiB Frames
    fn frame_alloc_n(&mut self, n: usize, align_huge_page: bool) -> Option<Frame>;
}

#[derive(Debug, Copy, Clone)]
pub struct MemoryRegion {
    start: mm::PhysicalAddress,
    size: usize,
    n_frames: usize,
    current: usize,
}

impl MemoryRegion {
    /// create a new memory region from start and end addresss.
    pub fn new(start: u64, end: u64) -> Self {
        let aligned_start = mm::Alignment::align_up(start, PageSize::Page4KiB.size());
        let aligned_end = mm::Alignment::align_down(end, PageSize::Page4KiB.size());
        let size = (aligned_end - aligned_start) as usize;
        let n_frames = size / PageSize::Page4KiB.size() as usize;
        MemoryRegion {
            start: mm::PhysicalAddress::from_u64(aligned_start),
            size,
            n_frames,
            current: 0,
        }
    }

    /// used fill the array with dummy values
    pub fn empty() -> Self {
        MemoryRegion {
            start: mm::PhysicalAddress::from_u64(0),
            size: 0,
            n_frames: 0,
            current: 0,
        }
    }

    /// returns the number of bytes free in this region
    #[inline]
    pub fn free_size(&self) -> usize {
        let current_size = self.current * PageSize::Page4KiB.size() as usize;
        self.size - current_size
    }

    /// returns the number of frames of 4KB size free
    #[inline]
    pub fn free_frames(&self) -> usize {
        self.n_frames - self.current
    }

    /// check whether N frames can be allocated here or not.
    #[inline]
    pub fn can_allocate(&self, n: usize) -> bool {
        self.current + n < self.n_frames
    }

    #[inline]
    pub fn can_allocate_aligned(&self, n: usize) -> bool {
        let offset = {
            let current_address =
                self.start.as_u64() + (self.current as u64 * PageSize::Page4KiB.size());
            let alignd_addr = mm::Alignment::align_up(current_address, PageSize::Page2MiB.size());
            ((alignd_addr - current_address) / PageSize::Page4KiB.size()) as usize
        };

        self.current + offset + n < self.n_frames
    }

    /// allocate N frames in the current region
    pub fn allocate_n(&mut self, n: usize, align_huge_page: bool) -> Option<Frame> {
        if !align_huge_page && !self.can_allocate(n) {
            return None;
        }

        if align_huge_page && !self.can_allocate_aligned(n) {
            return None;
        }

        if align_huge_page {
            // align the address at multiple of 2MiB
            let current_addr =
                self.start.as_u64() + (self.current as u64 * PageSize::Page4KiB.size());
            let aligned_addr = mm::Alignment::align_up(current_addr, PageSize::Page2MiB.size());
            let diff_addr = aligned_addr - current_addr;
            let offset_frames = (diff_addr / PageSize::Page4KiB.size()) as usize;
            self.current = self.current + n + offset_frames;
            return Some(Frame::from_address(mm::PhysicalAddress::from_u64(
                aligned_addr,
            )));
        } else {
            let current_address = mm::PhysicalAddress::from_u64(
                self.start.as_u64() + (self.current as u64 * PageSize::Page4KiB.size()),
            );
            self.current = self.current + n;
            return Some(Frame::from_address(current_address));
        }
    }
}

pub struct LinearFrameAllocator {
    pub memory_regions: [MemoryRegion; MAX_FREE_REGIONS],
    pub regions: usize,
}

impl LinearFrameAllocator {
    pub fn init() -> Self {
        let memory_map_opt = BootProtocol::get_memory_regions();
        if memory_map_opt.is_none() {
            panic!("Bootloader did not provide memory map.");
        }

        let memory_map = memory_map_opt.unwrap();
        // iterate over the memory map and prepare regions:
        let mut index = 0;
        let mut memory_regions = [MemoryRegion::empty(); MAX_FREE_REGIONS];

        for region in memory_map.iter() {
            if region.kind == MemoryRegionKind::Usable {
                log::debug!(
                    "Found memory region start=0x{:x}, end=0x{:x} as usable.",
                    region.start,
                    region.end
                );
                memory_regions[index] = MemoryRegion::new(region.start, region.end);
                index = index + 1;
            }
        }

        log::info!("Found {} memory regions as usable.", index + 1);
        LinearFrameAllocator {
            memory_regions,
            regions: index,
        }
    }
}

impl PhyFrameAllocator for LinearFrameAllocator {
    fn frame_alloc(&mut self) -> Option<Frame> {
        for region_idx in 0..self.regions {
            if self.memory_regions[region_idx].can_allocate(1) {
                let frame_opt = self.memory_regions[region_idx].allocate_n(1, false);
                return frame_opt;
            }
        }
        None
    }

    fn frame_dealloc(&mut self, index: usize) {
        log::warn!("Got index={}, Frame deallocation not implemented", index);
    }

    fn frame_alloc_n(&mut self, n: usize, align_huge_page: bool) -> Option<Frame> {
        for region_idx in 0..self.regions {
            let can_allocate = if !align_huge_page {
                self.memory_regions[region_idx].can_allocate(n)
            } else {
                self.memory_regions[region_idx].can_allocate_aligned(n)
            };
            if can_allocate {
                let frame_opt = self.memory_regions[region_idx].allocate_n(n, align_huge_page);
                return frame_opt;
            }
        }

        None
    }
}

lazy_static! {
    pub static ref LINEAR_ALLOCATOR: Mutex<LinearFrameAllocator> =
        Mutex::new(LinearFrameAllocator::init());
}

/// a function that lazy initializes LIEAR_ALLOCATOR
pub fn setup_physical_memory() {
    log::info!(
        "Set-up Linear memory allocator for Physical memory successfull, regions={}",
        LINEAR_ALLOCATOR.lock().regions
    );
}

pub struct PhysicalMemoryManager;

impl PhysicalMemoryManager {
    pub fn alloc() -> Option<Frame> {
        LINEAR_ALLOCATOR.lock().frame_alloc()
    }

    pub fn alloc_huge_page() -> Option<Frame> {
        let n_frames = (2 * mm::MemorySizes::OneMib as usize) / PageSize::Page4KiB.size() as usize;
        LINEAR_ALLOCATOR.lock().frame_alloc_n(n_frames, true)
    }

    pub fn free(_frame: Frame) {
        // Not implemented yet
        LINEAR_ALLOCATOR.lock().frame_dealloc(0);
    }
}
