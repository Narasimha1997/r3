extern crate alloc;
extern crate bit_field;
extern crate log;
extern crate spin;

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::mm::paging::{KernelVirtualMemoryManager, PageEntryFlags};
use crate::mm::{MemorySizes, VirtualAddress};

use core::mem;

// stack start
const STACK_ALLOCATOR_START_ADDR: u64 = 0x5fff00000000;
// stack end
const STACK_ALLOCATOR_END_ADDR: u64 = 0x600080000000;
pub const STACK_SIZE: usize = 2 * MemorySizes::OneMib as usize;

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
    /// Number of 2MiB stacks
    pub max_stacks: usize,
    /// Stacks that were freed last time.
    pub free_list: Vec<usize>,
    /// current number of stack space grown
    pub n_stacks: usize,
}

impl StackAllocator {
    /// create a new stack allocator instance
    pub fn new() -> StackAllocator {
        let max_stacks =
            (STACK_ALLOCATOR_END_ADDR - STACK_ALLOCATOR_START_ADDR) as usize / STACK_SIZE;
        StackAllocator {
            start_address: VirtualAddress::from_u64(STACK_ALLOCATOR_START_ADDR),
            max_stacks,
            free_list: Vec::new(),
            n_stacks: 0,
        }
    }

    #[inline]
    fn zero(&self, addr: VirtualAddress) {
        // TODO: find the fastest way to do this.
        let slice: &mut [u128; STACK_SIZE / mem::size_of::<u128>()] =
            unsafe { &mut *addr.get_mut_ptr() };

        for element in slice.iter_mut() {
            *element = 0;
        }
    }

    #[inline]
    fn addr_from_index(&self, index: usize) -> VirtualAddress {
        VirtualAddress::from_u64(STACK_ALLOCATOR_START_ADDR + (index * STACK_SIZE) as u64)
    }

    /// allocates a 4K stack and returns it's virtual address
    /// None if no space is available.
    pub fn alloc_stack(&mut self) -> Result<VirtualAddress, StackAllocatorError> {
        // try allocating from free list:
        if !self.free_list.is_empty() {
            // pop the last usize:
            let addr_index = self.free_list.pop();
            // use that stack for next allocation:
            let addr = self.addr_from_index(addr_index.unwrap());
            self.zero(addr);

            log::debug!(
                "Allocated stack from free-list index={:?}, n_remaining={}",
                addr_index, self.free_list.len()
            );

            return Ok(addr);
        }

        // free list is empty, allocate a new stack:
        if self.n_stacks >= self.max_stacks {
            panic!("Out of system stack memory.");
        }

        let at_addr = self.addr_from_index(self.n_stacks);
        // map this page:
        KernelVirtualMemoryManager::alloc_huge_page(at_addr, PageEntryFlags::kernel_hugepage_flags())
            .expect("Failed to allocate huge-page page for stack.");

        // zero the memory:
        self.zero(at_addr);
        self.n_stacks += 1;

        Ok(at_addr)
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
        let bounded_index = self.in_bounds(address);
        if bounded_index.is_none() {
            return Err(StackAllocatorError::InvalidAddress);
        }

        // return the stack:
        self.free_list.push(bounded_index.unwrap());
        Ok(())
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
