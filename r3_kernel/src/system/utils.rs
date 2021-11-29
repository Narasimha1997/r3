use crate::mm::stack::STACK_SIZE;

use crate::mm::{
    paging::KernelVirtualMemoryManager, paging::Page, paging::PageEntryFlags,
    paging::VirtualMemoryManager, phy::Frame, Alignment, MemorySizes, PhysicalAddress,
    VirtualAddress,
};

/// Area in which user code will be allocated
pub const USER_CODE_ADDRESS: u64 = 0x100000000;
/// Area in which user stack will be allocated
pub const USER_STACK_ADDRESS: u64 = 0x200000000;

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