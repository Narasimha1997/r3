use crate::mm;

extern crate alloc;
extern crate linked_list_allocator;
extern crate log;

// we are using LinkedListAllocator from osdev-rust comminity.
// Future plan is to use our own allocator.
use alloc::vec::Vec;
use linked_list_allocator::LockedHeap;

use crate::mm::paging;

pub const HEAP_START_ADDRESS: u64 = 0x7fff00000000;

// 10 MB of heap initially
pub const HEAP_SIZE: u64 = 10 * (mm::MemorySizes::OneMib as u64);

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

fn map_virtual_memory() {
    let n_4k_frames: usize = (HEAP_SIZE / paging::PageSize::Page4KiB.size()) as usize;
    log::debug!("n_heap_pages={}.", n_4k_frames);

    let heap_pages = paging::PageRange::new(
        mm::VirtualAddress::from_u64(HEAP_START_ADDRESS),
        (HEAP_SIZE / paging::PageSize::Page2MiB.size()) as usize,
        paging::PageSize::Page2MiB,
    );

    // map the virtual memory for heap:
    log::debug!(
        "Mapping kernel virtual memory for heap at 0x{:x}",
        HEAP_START_ADDRESS
    );

    let alloc_result = paging::KernelVirtualMemoryManager::alloc_huge_page_region(
        heap_pages,
        paging::PageEntryFlags::kernel_hugepage_flags(),
    );

    if alloc_result.is_err() {
        panic!(
            "Failed to allocate kernel heap, err={:?}",
            alloc_result.unwrap_err()
        );
    }

    log::info!("Allocated {}bytes at 0x{:x}", HEAP_SIZE, HEAP_START_ADDRESS);
}

pub fn init_heap() {
    map_virtual_memory();

    unsafe {
        KERNEL_HEAP_ALLOCATOR
            .lock()
            .init(HEAP_START_ADDRESS as usize, HEAP_SIZE as usize);
    }

    test_heap_alloc();
    log::info!("Setting up Kernel heap as Rust Global allocator is successful.");
}

fn test_heap_alloc() {
    log::debug!("Testing heap by allocating a vector: ");
    let mut test_vec: Vec<u64> = Vec::new();

    // insert some elements:
    test_vec.push(10);
    test_vec.push(20);
    test_vec.push(30);

    assert_eq!(test_vec.len(), 3);

    log::debug!("Test vector allocated at: {:p}", &test_vec[0]);
    core::mem::drop(test_vec);

    log::info!("Passed heap allocator test, successfully allocated and freed heap memory.");
}
