extern crate alloc;
extern crate object;

use alloc::vec::Vec;

use crate::mm::stack::STACK_SIZE;
use crate::system::filesystem::{vfs::FILESYSTEM, FSOps, FileDescriptor};
use crate::system::loader;

use core::{mem, ptr};
use object::{Object, ObjectSegment};

use crate::mm::{
    paging::KernelVirtualMemoryManager, paging::Page, paging::PageEntryFlags, paging::PageTable,
    paging::VirtualMemoryManager, phy::Frame, phy::PhysicalMemoryManager, Alignment, MemorySizes,
    PageTableLevel, PhysicalAddress, VirtualAddress,
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

/// Size of stack for each thread
pub const THREAD_STACK_SIZE: u64 = 2 * MemorySizes::OneMib as u64;

pub const USER_TEMP_STACK_MAPPING: u64 =  0x700000000000;

/// use huge pages to map heap
pub const USE_HUGEPAGE_HEAP: bool = true;

/// maximum file-descriptors that a process can have open any time
pub const MAX_FILE_DESCRIPTORS: usize = 512;

#[derive(Debug, Clone)]
pub enum ProcessError {
    StackOOM,
    StackOOB,
    StackAllocError,
    HeapOOM,
    HeapOOB,
    InvalidELF,
    MaxFDLimit,
    InvalidFD,
    CodeAllocationError,
}

#[derive(Debug, Clone)]
pub struct FDEntry {
    index: usize,
    pub fd: FileDescriptor,
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
    pub file_descriptors: Vec<FDEntry>,
    /// current fd allocated:
    pub fd_index: usize,
    /// proc code entrypoint
    pub code_entry: VirtualAddress,
    /// code page count - code segment uses 4KiB pages
    pub code_pages: u64,
    /// code references - number of child processes referencing the same section
    pub code_ref: u64,
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
        unmap_k: bool,
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

        // also map a kernel page
        if unmap_k {
            KernelVirtualMemoryManager::pt()
                .map_huge_page(
                    page,
                    alloc_result.unwrap(),
                    PageEntryFlags::user_hugepage_flags(),
                )
                .expect("Failed to map kernel page");
        } else {
            KernelVirtualMemoryManager::pt()
                .map_huge_page(
                    Page::from_address(VirtualAddress::from_u64(USER_TEMP_STACK_MAPPING)),
                    alloc_result.unwrap(),
                    PageEntryFlags::user_hugepage_flags(),
                )
                .expect("Failed to map kernel page");
        }

        if map_result.is_err() {
            log::error!(
                "Stack allocation failed, error={:?}",
                map_result.unwrap_err()
            );
            return Err(ProcessError::StackAllocError);
        }

        let vaddr = page.addr();

        // unmap it now
        if unmap_k {
            Self::zero(vaddr);
            KernelVirtualMemoryManager::pt()
                .unmap_page(page)
                .expect("Failed to unmap mapped page.");
        }

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

    #[inline]
    pub fn allocate_and_clone(
        child: &mut ProcessData,
        child_vmm: &mut VirtualMemoryManager,
        _rsp: u64,
    ) -> Result<VirtualAddress, ProcessError> {
        // allocate a new child stack
        let child_stk = Self::allocate_stack(child, child_vmm, false);
        if child_stk.is_err() {
            return child_stk;
        }

        let parent_stack_start = child.stack_space_start;
        let child_temp_start = VirtualAddress::from_u64(USER_TEMP_STACK_MAPPING);

        // copy
        unsafe {
            let parent_ptr = parent_stack_start.get_ptr::<u8>();
            let child_ptr = child_temp_start.get_mut_ptr::<u8>();

            // copy
            ptr::copy_nonoverlapping(parent_ptr, child_ptr, STACK_SIZE as usize);
        }

        // late unmap the kernel region
        KernelVirtualMemoryManager::pt()
            .unmap_page(Page::from_address(child_temp_start))
            .expect("Failed to unmap mapped page.");

        Ok(parent_stack_start)
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
    pub fn contract(proc_vmm: &mut ProcessData, size: usize) -> Result<usize, ProcessError> {
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

    #[inline]
    pub fn current_size(proc_data: &mut ProcessData) -> u64 {
        if USE_HUGEPAGE_HEAP {
            (proc_data.heap_pages as u64) * (MemorySizes::OneMib as u64 * 2)
        } else {
            (proc_data.heap_pages as u64) * (MemorySizes::OneKiB as u64 * 4)
        }
    }

    #[inline]
    pub fn current_end_address(proc_data: &mut ProcessData) -> VirtualAddress {
        let size = Self::current_size(proc_data) as u64;
        VirtualAddress::from_u64(proc_data.heap_start.as_u64() + size)
    }

    #[inline]
    pub fn set_break_at(
        proc_data: &mut ProcessData,
        vmm: &mut VirtualMemoryManager,
        addr: VirtualAddress,
    ) -> Result<(), ProcessError> {
        let mut aligned_addr = addr.clone();
        if USE_HUGEPAGE_HEAP {
            aligned_addr.align_up(2 * MemorySizes::OneMib as u64);
        } else {
            aligned_addr.align_up(4 * MemorySizes::OneKiB as u64);
        }

        if aligned_addr.as_u64() < proc_data.heap_start.as_u64() {
            return Err(ProcessError::HeapOOB);
        }

        // expand or contract?
        let current_end_addr = Self::current_end_address(proc_data);
        if current_end_addr.as_u64() > aligned_addr.as_u64() {
            // contract
            let size = current_end_addr.as_u64() - aligned_addr.as_u64();
            let contract_res = Self::contract(proc_data, size as usize);
            if contract_res.is_err() {
                return Err(contract_res.unwrap_err());
            }
        } else {
            let size = aligned_addr.as_u64() - current_end_addr.as_u64();
            let expand_res = Self::expand(proc_data, vmm, size as usize);
            if expand_res.is_err() {
                return Err(expand_res.unwrap_err());
            }
        }

        Ok(())
    }
}

pub struct CodeMapper;

impl CodeMapper {
    #[inline]
    pub fn load_elf(
        proc_vmm: &mut ProcessData,
        vmm: &mut VirtualMemoryManager,
        path: &str,
    ) -> Result<(), ProcessError> {
        let file_buffer_res = loader::read_executable(&path);
        if file_buffer_res.is_err() {
            log::error!("{:?}", file_buffer_res.unwrap_err());
            return Err(ProcessError::InvalidELF);
        }

        let file_buffer = file_buffer_res.unwrap();
        // map this buffer as ELF
        let buffer_ref = &file_buffer[0..];
        let elf_result = object::File::parse(buffer_ref);

        if elf_result.is_err() {
            log::error!("ELF Loader Error {:?}", elf_result.unwrap_err());
            return Err(ProcessError::InvalidELF);
        }

        let elf = elf_result.unwrap();
        let mut total_pages = 0;

        // map all the segments:
        for segment in elf.segments() {
            log::debug!(
                "{} allocation section={:?} at=0x{:x}, size={}",
                path,
                segment.name(),
                segment.address(),
                segment.size()
            );

            let section_start = segment.address();
            let aligned_sec_start =
                Alignment::align_down(section_start, 4 * MemorySizes::OneKiB as u64);
            let aligned_size = Alignment::align_up(segment.size(), 4 * MemorySizes::OneKiB as u64);

            let n_pages = aligned_size / (4 * MemorySizes::OneKiB as u64);
            total_pages = total_pages + n_pages;

            for i in 0..n_pages {
                // map kernel and user pages
                let frame = PhysicalMemoryManager::alloc().expect("RAM OOM");
                let page = Page::from_address(VirtualAddress::from_u64(
                    aligned_sec_start + (i * 4 * MemorySizes::OneKiB as u64),
                ));
                KernelVirtualMemoryManager::pt()
                    .map_page(page, frame, PageEntryFlags::user_flags())
                    .expect("Failed to map kernel page while mapping code.");
                vmm.map_page(page, frame, PageEntryFlags::user_flags())
                    .expect("Failed to map user page while mapping code");
            }

            if let Ok(data) = segment.data() {
                // zero this layout
                let start_ptr = VirtualAddress::from_u64(aligned_sec_start).get_mut_ptr::<u8>();
                unsafe {
                    ptr::write_bytes(
                        start_ptr,
                        0,
                        n_pages as usize * 4 * MemorySizes::OneKiB as usize,
                    );
                    // write data
                    ptr::copy_nonoverlapping(
                        data.as_ptr(),
                        start_ptr.add((segment.address() - aligned_sec_start) as usize),
                        segment.size() as usize,
                    );
                }
            }

            // unmap kernel entries:
            for i in 0..n_pages {
                let page = Page::from_address(VirtualAddress::from_u64(
                    aligned_sec_start + (i * 4 * MemorySizes::OneKiB as u64),
                ));
                KernelVirtualMemoryManager::pt()
                    .unmap_page(page)
                    .expect("Failed to unmap mapped kernel pages.");
            }
        }

        let entry_addr = elf.entry();
        proc_vmm.code_entry = VirtualAddress::from_u64(entry_addr);
        proc_vmm.code_pages = total_pages;

        // mark the end of heap as 2MiB aligned page
        let aligned_hugepage_size = Alignment::align_up(
            total_pages * 4 * MemorySizes::OneKiB as u64,
            2 * MemorySizes::OneMib as u64,
        );
        proc_vmm.heap_start = VirtualAddress::from_u64(aligned_hugepage_size);

        Ok(())
    }

    #[inline]
    pub fn share_pages(
        parent: &mut ProcessData,
        child: &mut ProcessData,
        parent_vmm: &mut VirtualMemoryManager,
        child_vmm: &mut VirtualMemoryManager,
    ) {
        if parent.code_pages == 0 {
            return;
        }

        // map the top level table
        let code_addr = VirtualAddress::from_u64(USER_CODE_ADDRESS);

        // map the page
        let l4_index = code_addr.get_level_index(PageTableLevel::Level4);
        // clone this index:
        let child_pt: &mut PageTable = unsafe { &mut *child_vmm.l4_virtual_address.get_mut_ptr() };
        let parent_pt: &PageTable = unsafe { &*parent_vmm.l4_virtual_address.get_ptr() };

        child_pt.entries[l4_index.as_usize()] = parent_pt.entries[l4_index.as_usize()].clone();

        child.code_ref = parent.code_ref + 1;
        child.code_pages = parent.code_pages;
        child.heap_start = child.heap_start;
        child.heap_pages = child.heap_pages;
    }
}

pub struct ProcessFDPool;

impl ProcessFDPool {
    #[inline]
    pub fn put(proc_data: &mut ProcessData, fd: FileDescriptor) -> Result<usize, ProcessError> {
        if proc_data.file_descriptors.len() + 1 > MAX_FILE_DESCRIPTORS {
            return Err(ProcessError::MaxFDLimit);
        }

        let index = proc_data.fd_index;
        let fd_entry = FDEntry { index, fd };
        proc_data.fd_index = index + 1;
        proc_data.file_descriptors.push(fd_entry);
        Ok(index)
    }

    #[inline]
    pub fn get_mut(proc_data: &mut ProcessData, fd_index: usize) -> Option<&mut FDEntry> {
        for idx in 0..proc_data.file_descriptors.len() {
            if proc_data.file_descriptors[idx].index == fd_index {
                return proc_data.file_descriptors.get_mut(idx);
            }
        }

        None
    }

    #[inline]
    pub fn clone(parent: &mut ProcessData, child: &mut ProcessData) {
        for fd in parent.file_descriptors.iter() {
            child.file_descriptors.push(fd.clone());
        }

        child.fd_index = parent.fd_index;
    }

    #[inline]
    pub fn remove(proc_data: &mut ProcessData, fd_index: usize) -> Result<(), ProcessError> {
        for idx in 0..proc_data.file_descriptors.len() {
            if proc_data.file_descriptors[idx].index == fd_index {
                proc_data.file_descriptors.remove(idx);
                return Ok(());
            }
        }

        Err(ProcessError::InvalidFD)
    }
}

pub fn create_default_descriptors(proc_data: &mut ProcessData) {
    let dev_fd = FILESYSTEM
        .lock()
        .open("/dev/serial", 0)
        .expect("/dev/serial not found on this platform, cannot create process stdout.");

    let stdin = dev_fd.clone();
    let stdout = dev_fd.clone();
    let stderr = dev_fd;

    ProcessFDPool::put(proc_data, stdin).expect("Failed to create default stdin");
    ProcessFDPool::put(proc_data, stdout).expect("Failed to create default stdin");
    ProcessFDPool::put(proc_data, stderr).expect("Failed to create default stdin");
}

pub fn create_cloned_layout(
    parent: &mut ProcessData,
    parent_vmm: &mut VirtualMemoryManager,
    child_vmm: &mut VirtualMemoryManager,
) -> ProcessData {
    // create an empty layout
    let stack_space_start = VirtualAddress::from_u64(USER_VIRT_END - PROCESS_STACKS_SIZE);

    let mut proc_data = ProcessData {
        stack_space_start,
        free_stack_holes: Vec::new(),
        n_stacks: 0,
        heap_pages: 0,
        heap_start: VirtualAddress::from_u64(0),
        heap_alloc_pages: 0,
        max_heap_pages: 0,
        file_descriptors: Vec::new(),
        fd_index: 0,
        code_entry: VirtualAddress::from_u64(0),
        code_pages: 0,
        code_ref: 0,
    };

    CodeMapper::share_pages(parent, &mut proc_data, parent_vmm, child_vmm);
    ProcessFDPool::clone(parent, &mut proc_data);
    proc_data
}

pub fn create_process_layout(path: &str, vmm: &mut VirtualMemoryManager) -> ProcessData {
    // create an empty layout
    let stack_space_start = VirtualAddress::from_u64(USER_VIRT_END - PROCESS_STACKS_SIZE);

    let mut proc_data = ProcessData {
        stack_space_start,
        free_stack_holes: Vec::new(),
        n_stacks: 0,
        heap_pages: 0,
        heap_start: VirtualAddress::from_u64(0),
        heap_alloc_pages: 0,
        max_heap_pages: 0,
        file_descriptors: Vec::new(),
        fd_index: 0,
        code_entry: VirtualAddress::from_u64(0),
        code_pages: 0,
        code_ref: 0,
    };

    // create the code segment
    let code_alloc_result = CodeMapper::load_elf(&mut proc_data, vmm, &path);
    if code_alloc_result.is_err() {
        panic!("Failed to allocate code for the process.");
    }

    let max_heap_size = stack_space_start.as_u64() - proc_data.heap_start.as_u64();
    let max_heap_pages = if USE_HUGEPAGE_HEAP {
        max_heap_size / (4 * MemorySizes::OneMib as u64)
    } else {
        max_heap_size / (4 * MemorySizes::OneKiB as u64)
    };

    proc_data.max_heap_pages = max_heap_pages;
    proc_data
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
