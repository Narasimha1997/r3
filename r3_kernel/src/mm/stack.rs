extern crate log;
extern crate spin;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::mm::paging::{KernelVirtualMemoryManager, PageEntryFlags, PageRange, PageSize};
use crate::mm::{MemorySizes, VirtualAddress};

use core::mem;

const STACK_ALLOCATOR_START_ADDR: u64 = 0x5fff00000000;
pub const STACK_SIZE: usize = 2 * MemorySizes::OneMib as usize;
const MAX_STACKS: usize = 32;

#[derive(Debug)]
pub enum StackAllocatorError {
    OOM,
    InvalidAddress,
}

/// A global stack allocator
/// The goal of this allocator is to lease 8k stack space for each of the
/// threads being created.
pub struct StackAllocator {
    /// Virtual start address of the stack
    pub start_address: VirtualAddress,
    pub next_to_allocate: usize,
    pub recently_freed: usize,
    pub n_stacks: usize,
}

impl StackAllocator {
    /// create a new stack allocator instance
    pub fn new() -> StackAllocator {
        let stack_size_bytes = MAX_STACKS * STACK_SIZE;
        log::info!("Allocating {}bytes for stack.", stack_size_bytes);

        // allocate huge pages at this address:
        let start_addr = VirtualAddress::from_u64(STACK_ALLOCATOR_START_ADDR);
        let page_range = PageRange::new(
            start_addr,
            stack_size_bytes / PageSize::Page2MiB.size() as usize,
            PageSize::Page2MiB,
        );

        // allocate memory and virtual map it:
        let result = KernelVirtualMemoryManager::alloc_huge_page_region(
            page_range,
            PageEntryFlags::kernel_hugepage_flags(),
        );

        if result.is_err() {
            panic!("Failed to allocate stack memory, {:?}", result.unwrap_err());
        }

        log::info!(
            "Allocated memory for stack allocator, size={}bytes",
            stack_size_bytes
        );

        StackAllocator {
            start_address: start_addr,
            next_to_allocate: 0,
            recently_freed: 0,
            n_stacks: MAX_STACKS,
        }
    }

    #[inline]
    fn zero(&self, addr: VirtualAddress) {
        let slice: &mut [u64; STACK_SIZE / mem::size_of::<u64>()] =
            unsafe { &mut *addr.get_mut_ptr() };

        for element in slice.iter_mut() {
            *element = 0;
        }
    }

    /// allocates a 4K stack and returns it's virtual address
    /// None if no space is available.
    pub fn alloc_stack(&mut self) -> Result<VirtualAddress, StackAllocatorError> {
        if self.next_to_allocate == self.recently_freed {
            if self.next_to_allocate >= self.n_stacks {
                return Err(StackAllocatorError::OOM);
            }

            // a new unseen stack is to be returned.
            let address_u64 =
                self.start_address.as_u64() + (self.next_to_allocate * STACK_SIZE) as u64;
            self.next_to_allocate += 1;
            self.recently_freed = self.next_to_allocate;

            let vaddr = VirtualAddress::from_u64(address_u64);
            self.zero(vaddr);
            return Ok(vaddr);
        } else {
            // re-use the recently freed stack location:
            let address_u64 =
                self.start_address.as_u64() + (self.recently_freed * STACK_SIZE) as u64;
            self.recently_freed = self.next_to_allocate;
            let vaddr = VirtualAddress::from_u64(address_u64);
            self.zero(vaddr);
            return Ok(vaddr);
        }
    }

    #[inline]
    fn in_bounds(&self, address: VirtualAddress) -> Option<usize> {
        if address.as_u64() < self.start_address.as_u64() {
            return None;
        }

        if address.as_u64() > (self.start_address.as_u64() + (self.n_stacks * STACK_SIZE) as u64) {
            return None;
        }

        return Some((address.as_u64() - self.start_address.as_u64()) as usize / STACK_SIZE);
    }

    pub fn free_stack(&mut self, address: VirtualAddress) -> Result<(), StackAllocatorError> {
        if let Some(index) = self.in_bounds(address) {
            self.recently_freed = index;
            return Ok(());
        }

        return Err(StackAllocatorError::InvalidAddress);
    }
}

lazy_static! {
    pub static ref STACK_ALLOCATOR: Mutex<StackAllocator> = Mutex::new(StackAllocator::new());
}

/// init stack allocator
pub fn setup_stack_allocator() {
    log::info!(
        "StackAllocator successfully set-up, address=0x{:x}",
        STACK_ALLOCATOR.lock().start_address.as_u64()
    );
}
