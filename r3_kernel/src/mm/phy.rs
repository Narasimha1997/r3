extern crate bootloader;
extern crate log;
extern crate spin;

use crate::boot_proto::BootProtocol;
use crate::mm;
use crate::mm::paging::{PageSize, PagingError};
use crate::mm::MemorySizes;
use bootloader::boot_info::{MemoryRegionKind, MemoryRegions};

use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Frame(mm::PhysicalAddress);

const MAX_FREE_REGIONS: usize = 64;

/// Following X bytes are allocated for DMA memory.
const DMA_REGION_SIZE: usize = 2 * MemorySizes::OneMib as usize;
const DMA_FRAME_SIZE: usize = MemorySizes::OneKiB as usize * 8;

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
    #[inline]
    fn create_combined_regions(
        boot_regions: &MemoryRegions,
        os_regions: &mut [MemoryRegion],
    ) -> usize {
        let mut current_start: u64 = 0;
        let mut current_end: u64 = 0;
        let mut n_regions = 0;

        for idx in 0..boot_regions.len() {
            let region = &boot_regions[idx];
            // ignore the region below 4K
            if region.end <= 4096 {
                continue;
            }

            if region.kind == MemoryRegionKind::Usable {
                if current_start == 0 && current_end == 0 {
                    current_start = region.start;
                    current_end = region.end;
                }
                if current_end == region.start {
                    // linear
                    current_end = region.end;
                } else {
                    // non-linear
                    let memory_region = MemoryRegion::new(current_start, current_end);
                    log::info!(
                        "Found memory region of size: {} bytes. start=0x{:x}, end=0x{:x}",
                        current_end - current_start,
                        current_start,
                        current_end
                    );
                    os_regions[n_regions] = memory_region;
                    n_regions += 1;

                    // re-init start and end
                    current_start = region.start;
                    current_end = region.end;
                }
            }
        }

        n_regions
    }

    pub fn init() -> Self {
        let memory_map_opt = BootProtocol::get_memory_regions();
        if memory_map_opt.is_none() {
            panic!("Bootloader did not provide memory map.");
        }

        let memory_map = memory_map_opt.unwrap();
        // iterate over the memory map and prepare regions:
        let mut memory_regions = [MemoryRegion::empty(); MAX_FREE_REGIONS];

        let n_regions = Self::create_combined_regions(memory_map, &mut memory_regions);

        log::info!("Found {} memory regions as usable.", n_regions);
        LinearFrameAllocator {
            memory_regions,
            regions: n_regions,
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

#[derive(Debug, Clone, Copy)]
pub struct DMABuffer {
    pub phy_addr: mm::PhysicalAddress,
    pub virt_addr: mm::VirtualAddress,
    pub size: usize,
}

impl DMABuffer {
    #[inline]
    pub fn new(phy_addr: mm::PhysicalAddress, size: usize) -> Self {
        DMABuffer {
            phy_addr,
            virt_addr: mm::p_to_v(phy_addr),
            size,
        }
    }

    #[inline]
    pub fn get_ptr<T>(&self) -> *const T {
        self.virt_addr.as_u64() as *const T
    }

    #[inline]
    pub fn get_mut_ptr<T>(&self) -> *mut T {
        self.virt_addr.as_u64() as *mut T
    }
}

/// Manages memory allocated for DMA purposes.
/// The granularity of memory allocation is 8KiB frames.
/// This memory cannot be freed once allocated
/// The devices need to lock this memory area during init time
/// by stating it's requirements.
pub struct DMAAllocator {
    pub max_frames: usize,
    pub current_index: usize,
    pub start_addr: mm::PhysicalAddress,
}

impl DMAAllocator {
    pub fn empty() -> Self {
        // is there a free region below 16MiB?
        let mut alloc_lock: MutexGuard<LinearFrameAllocator> = LINEAR_ALLOCATOR.lock();
        for region in alloc_lock.memory_regions.iter_mut() {
            if region.start.as_u64() < 16 * MemorySizes::OneMib as u64 {
                let dma_start = region.start;
                let dma_end =
                    mm::PhysicalAddress::from_u64(dma_start.as_u64() + DMA_REGION_SIZE as u64);
                region.start = dma_end;
                region.size = region.size - DMA_REGION_SIZE;
                region.n_frames = (region.size) / (4 * MemorySizes::OneKiB as usize);

                log::debug!(
                    "Moved the start address of memory region below 16MiB from 0x{:x} to 0x{:x}",
                    dma_start.as_u64(),
                    region.start.as_u64()
                );

                let aligned_start =
                    mm::Alignment::align_up(dma_start.as_u64(), DMA_FRAME_SIZE as u64);

                let aligned_end =
                    mm::Alignment::align_down(dma_end.as_u64(), DMA_FRAME_SIZE as u64);
                let max_frames = (aligned_end - aligned_start) / (DMA_FRAME_SIZE as u64);

                return DMAAllocator {
                    max_frames: max_frames as usize,
                    current_index: 0,
                    start_addr: mm::PhysicalAddress::from_u64(aligned_start),
                };
            }
        }

        panic!("DMA Region could not be found.")
    }

    #[inline]
    pub fn alloc(&mut self, size: usize) -> Option<DMABuffer> {
        let aligned_size = mm::Alignment::align_up(size as u64, DMA_FRAME_SIZE as u64) as usize;
        let n_frames = aligned_size / DMA_FRAME_SIZE;

        if self.current_index + n_frames > self.max_frames {
            return None;
        }

        let start_addr = mm::PhysicalAddress::from_u64(
            self.start_addr.as_u64() + (self.current_index * DMA_FRAME_SIZE) as u64,
        );

        self.current_index += n_frames;
        Some(DMABuffer::new(start_addr, size))
    }
}

lazy_static! {
    pub static ref DMA_ALLOCATOR: Mutex<DMAAllocator> = Mutex::new(DMAAllocator::empty());
}

pub struct DMAMemoryManager;

impl DMAMemoryManager {
    pub fn alloc(size: usize) -> Option<DMABuffer> {
        DMA_ALLOCATOR.lock().alloc(size)
    }
}

/// a function that lazy initializes LIEAR_ALLOCATOR
pub fn setup_physical_memory() {
    log::info!(
        "Set-up Linear memory allocator for Physical memory successfull, regions={}",
        LINEAR_ALLOCATOR.lock().regions
    );

    let dma_lock = DMA_ALLOCATOR.lock();

    log::info!(
        "Set-up DMA Allocator for DMA memory manager successfull, start=0x{:x}, max_frames={}",
        dma_lock.start_addr.as_u64(),
        dma_lock.max_frames
    );
}
