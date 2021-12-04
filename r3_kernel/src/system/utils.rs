extern crate alloc;

use alloc::vec::Vec;

use crate::mm::stack::STACK_SIZE;
use crate::system::filesystem::FileDescriptor;

use core::mem;

use crate::mm::{
    paging::KernelVirtualMemoryManager, paging::Page, paging::PageEntryFlags,
    paging::VirtualMemoryManager, phy::Frame, phy::PhysicalMemoryManager, Alignment, MemorySizes,
    PhysicalAddress, VirtualAddress,
};

/// Area in which user code will be allocated
pub const USER_CODE_ADDRESS: u64 = 0x400000;
/// Area in which user stack will be allocated
pub const USER_STACK_ADDRESS: u64 = 0x200000000;

// process layout -
// | --- .text, .bss, .data ---- | ---- heap memory ----- | ---- stacks for each thread ------|
// argv and envs |

/// Start of the user virtual address space
pub const USER_VIRT_START: u64 = 0;

/// End of the user virtual address space
pub const USER_VIRT_END: u64 = 0x800000000000;

/// Stack area size, stacks are allocated as follows:
/// Each stack will be of size 2MiB follwed by a 2MiB virtual gap
/// So each allocation will cost 2MiB physically and 4MiB virtually
pub const PROCESS_STACKS_SIZE: u64 = 16 * MemorySizes::OneGiB as u64;

pub const THREAD_STACK_SIZE: u64 = 2 * MemorySizes::OneMib as u64;

pub const USE_HUGEPAGE_HEAP: bool = true;

#[derive(Debug, Clone)]
pub enum ProcessError {
    StackOOM,
    StackOOB,
    StackAllocError,
    HeapOOM,
    HeapOOB,
}

#[derive(Debug, Clone)]
pub struct ProcessData {
    /// current allocated page allocated stack
    pub stack_space_start: VirtualAddress,
    /// contains 2MiB holes in stack space, will be reallocated from this hole
    /// upon request if any.
    pub free_stack_holes: Vec<u64>,
    /// current allocated stack end, will be incremented by 2 for each
    /// allocation.
    pub n_stacks: u64,
    /// start address from where process heap is allocated
    pub heap_start: VirtualAddress,
    /// current heap size
    pub heap_pages: u64,
    /// number of pages already allocated
    pub heap_alloc_pages: u64,
    /// max heap pages
    pub max_heap_pages: u64,
    /// list of open file descriptors for the process
    pub file_descriptors: Vec<FileDescriptor>,
    /// proc code entrypoint
    pub code_entry: VirtualAddress,
}

pub struct ProcessStackManager;

impl ProcessStackManager {
    #[inline]
    fn zero(addr: VirtualAddress) {
        // TODO: find the fastest way to do this.
        let slice: &mut [u128; THREAD_STACK_SIZE as usize / mem::size_of::<u128>()] =
            unsafe { &mut *addr.get_mut_ptr() };

        for element in slice.iter_mut() {
            *element = 0;
        }
    }

    #[inline]
    pub fn allocate_stack(
        proc_data: &mut ProcessData,
        vmm: &mut VirtualMemoryManager,
    ) -> Result<VirtualAddress, ProcessError> {
        // is there a free stack in the pool?
        if proc_data.free_stack_holes.len() > 0 {
            let stk_index = proc_data.free_stack_holes.pop().unwrap();
            // return it's address:
            let vaddr = VirtualAddress::from_u64(
                proc_data.stack_space_start.as_u64() + (stk_index * 4 * MemorySizes::OneMib as u64),
            );

            Self::zero(vaddr);
            return Ok(vaddr);
        }

        // allocate a new 2MiB stack:
        let alloc_result = PhysicalMemoryManager::alloc_huge_page();
        if alloc_result.is_none() {
            log::error!("Stack allocation failed, out of memory!");
            return Err(ProcessError::StackOOM);
        }

        let page = Page::from_address(VirtualAddress::from_u64(
            proc_data.stack_space_start.as_u64()
                + proc_data.n_stacks * 4 * MemorySizes::OneMib as u64,
        ));

        let map_result = vmm.map_huge_page(
            page,
            alloc_result.unwrap(),
            PageEntryFlags::user_hugepage_flags(),
        );

        if map_result.is_err() {
            log::error!(
                "Stack allocation failed, error={:?}",
                map_result.unwrap_err()
            );
            return Err(ProcessError::StackAllocError);
        }

        let vaddr = page.addr();
        Self::zero(vaddr);

        // increment the counter
        proc_data.n_stacks += 2;
        Ok(vaddr)
    }

    #[inline]
    pub fn free_stack(
        proc_data: &mut ProcessData,
        addr: VirtualAddress,
    ) -> Result<(), ProcessError> {
        // check out of bounds
        if addr.as_u64() > (proc_data.stack_space_start.as_u64() + STACK_SIZE as u64) {
            return Err(ProcessError::StackOOB);
        }

        let aligned_loc = Alignment::align_down(addr.as_u64(), 4 * MemorySizes::OneMib as u64);
        let nth = aligned_loc / (4 * MemorySizes::OneMib as u64);

        proc_data.free_stack_holes.push(nth);
        Ok(())
    }
}

pub struct ProcessHeapAllocator;

impl ProcessHeapAllocator {
    #[inline]
    pub fn expand(
        proc_vmm: &mut ProcessData,
        vmm: &mut VirtualMemoryManager,
        size: usize,
    ) -> Result<usize, ProcessError> {
        // align the size to page sized blocks
        let align_size = if USE_HUGEPAGE_HEAP {
            2 * MemorySizes::OneMib as u64
        } else {
            4 * MemorySizes::OneKiB as u64
        };

        // align this address:
        let aligned_size = Alignment::align_up(size as u64, align_size);
        // allocate the heap
        let mut n_pages = aligned_size / align_size;

        // can we re-use already allocated pages?
        if proc_vmm.heap_pages + n_pages <= proc_vmm.heap_alloc_pages {
            let current_pages = proc_vmm.heap_pages;
            proc_vmm.heap_pages = proc_vmm.heap_pages + n_pages;
            return Ok((current_pages * align_size) as usize);
        }

        n_pages = n_pages + proc_vmm.heap_pages - proc_vmm.heap_alloc_pages;
        proc_vmm.heap_pages = proc_vmm.heap_alloc_pages + n_pages;
        let current_end = proc_vmm.heap_alloc_pages * align_size;
        let mut new_addr = proc_vmm.heap_start.as_u64() + current_end;

        if proc_vmm.heap_alloc_pages + n_pages > proc_vmm.max_heap_pages {
            return Err(ProcessError::HeapOOM);
        }

        for _ in 0..n_pages {
            let new_page = Page::from_address(VirtualAddress::from_u64(new_addr));
            let alloc_result = if USE_HUGEPAGE_HEAP {
                let huge_frame_res = PhysicalMemoryManager::alloc_huge_page();
                if huge_frame_res.is_none() {
                    return Err(ProcessError::HeapOOM);
                }
                let huge_frame = huge_frame_res.unwrap();
                new_addr = new_addr + align_size;
                vmm.map_huge_page(new_page, huge_frame, PageEntryFlags::user_hugepage_flags())
            } else {
                let frame_res = PhysicalMemoryManager::alloc();
                if frame_res.is_none() {
                    return Err(ProcessError::HeapOOM);
                }
                let frame = frame_res.unwrap();
                new_addr = new_addr + align_size;
                vmm.map_page(new_page, frame, PageEntryFlags::user_flags())
            };

            if alloc_result.is_err() {
                log::error!("Failed to expand heap.");
            }
        }

        proc_vmm.heap_alloc_pages = proc_vmm.heap_alloc_pages + n_pages;
        Ok(current_end as usize)
    }

    #[inline]
    pub fn contract(
        proc_vmm: &mut ProcessData,
        size: usize,
    ) -> Result<usize, ProcessError> {
        let align_size = if USE_HUGEPAGE_HEAP {
            2 * MemorySizes::OneMib as u64
        } else {
            4 * MemorySizes::OneKiB as u64
        };
        let aligned_size = Alignment::align_up(size as u64, align_size);
        let n_pages = aligned_size / align_size;

        if n_pages > proc_vmm.heap_pages {
            return Err(ProcessError::HeapOOB);
        }

        let current_end = proc_vmm.heap_pages * align_size;
        proc_vmm.heap_pages = proc_vmm.heap_pages - n_pages;
        Ok(current_end as usize)
    }
}

pub fn map_user_stack(
    stack_addr: VirtualAddress,
    n_current_threads: usize,
    proc_vmm: &mut VirtualMemoryManager,
) -> VirtualAddress {
    // maps the stack address to user code's stack location
    // using huge pages
    let new_stack_address =
        VirtualAddress::from_u64(USER_STACK_ADDRESS + (n_current_threads * STACK_SIZE) as u64);
    // map the stack to it's virtual address:
    let stack_phy_address = KernelVirtualMemoryManager::pt().translate(stack_addr);
    if stack_phy_address.is_none() {
        panic!("Incosistent memory state while allocating thread.");
    }

    log::info!("Mapping user stack.");

    // map this physical address to given new virtual address as a 2MiB Page
    let res = proc_vmm.map_huge_page(
        Page::from_address(new_stack_address),
        Frame::from_address(stack_phy_address.unwrap()),
        PageEntryFlags::user_hugepage_flags(),
    );

    if res.is_err() {
        panic!("{:?}", res);
    }
    return new_stack_address;
}

pub fn map_user_code(
    func_addr: VirtualAddress,
    proc_vmm: &mut VirtualMemoryManager,
) -> VirtualAddress {
    let func_phy_addr = KernelVirtualMemoryManager::pt()
        .translate(func_addr)
        .unwrap();

    let base_aligned_addr =
        Alignment::align_down(func_phy_addr.as_u64(), MemorySizes::OneKiB as u64 * 4);

    log::info!(
        "Func phy addr: 0x{:x}, aligned: 0x{:x}",
        func_phy_addr.as_u64(),
        base_aligned_addr
    );

    let offset = func_phy_addr.as_u64() - base_aligned_addr;
    let code_base_addr = VirtualAddress::from_u64(USER_CODE_ADDRESS);
    // map this to virtual memory region:

    log::info!("Mapping user code");
    proc_vmm
        .map_page(
            Page::from_address(code_base_addr),
            Frame::from_address(PhysicalAddress::from_u64(base_aligned_addr)),
            PageEntryFlags::user_flags(),
        )
        .expect("Failed to map codebase address for user thread.");
    let gaurd_frame = base_aligned_addr + (4 * MemorySizes::OneKiB as u64);

    // map this extra page:
    proc_vmm
        .map_page(
            Page::from_address(VirtualAddress::from_u64(
                code_base_addr.as_u64() + (4 * MemorySizes::OneKiB as u64),
            )),
            Frame::from_address(PhysicalAddress::from_u64(gaurd_frame)),
            PageEntryFlags::user_flags(),
        )
        .expect("Gaurd page allocation error");
    // return the code address:
    VirtualAddress::from_u64(code_base_addr.as_u64() + offset)
}
