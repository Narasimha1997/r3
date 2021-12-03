extern crate alloc;

use alloc::vec::Vec;

use crate::mm::stack::STACK_SIZE;
use crate::system::filesystem::FileDescriptor;

use core::mem;

use crate::mm::{
    paging::KernelVirtualMemoryManager, paging::Page, paging::PageEntryFlags,
    paging::VirtualMemoryManager, phy::Frame, phy::PhysicalMemoryManager, Alignment,
    MemorySizes, PhysicalAddress, VirtualAddress,
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
    pub heap_size: u64,
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
    ) -> Option<VirtualAddress> {
        // is there a free stack in the pool?
        if proc_data.free_stack_holes.len() > 0 {
            let stk_index = proc_data.free_stack_holes.pop().unwrap();
            // return it's address:
            let vaddr = VirtualAddress::from_u64(
                proc_data.stack_space_start.as_u64() + (stk_index * 4 * MemorySizes::OneMib as u64),
            );

            Self::zero(vaddr);
            return Some(vaddr);
        }

        // allocate a new 2MiB stack:
        let alloc_result = PhysicalMemoryManager::alloc_huge_page();
        if alloc_result.is_none() {
            log::error!("Stack allocation failed, out of memory!");
            return None;
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
            return None;
        }

        let vaddr = page.addr();
        Self::zero(vaddr);

        // increment the counter
        proc_data.n_stacks += 2;
        Some(vaddr)
    }

    #[inline]
    pub fn free_stack(proc_data: &mut ProcessData, addr: VirtualAddress) {
        let aligned_loc = Alignment::align_down(addr.as_u64(), 4 * MemorySizes::OneMib as u64);
        let nth = aligned_loc / (4 * MemorySizes::OneMib as u64);

        proc_data.free_stack_holes.push(nth);
    }
}

pub struct ProcessHeapAllocator;


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
