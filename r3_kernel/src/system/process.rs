extern crate alloc;
extern crate log;
extern crate spin;

use crate::cpu::mmu;
use crate::mm::paging::{KernelVirtualMemoryManager, PageTable, VirtualMemoryManager};
use crate::mm::phy::PhysicalMemoryManager;
use crate::mm::{PhysicalAddress, VirtualAddress};
use crate::system::thread::ThreadID;

use lazy_static::lazy_static;

use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

static CURRENT_PID: AtomicU64 = AtomicU64::new(0);

pub fn new_pid() -> PID {
    let current = CURRENT_PID.load(Ordering::SeqCst);
    if current + 1 == u64::max_value() {
        panic!("Could not create process, Out of PIDs");
    }

    CURRENT_PID.store(current + 1, Ordering::SeqCst);
    PID(current)
}

#[derive(Debug)]
pub enum ProcessError {
    UnknownThreadID,
    UnknwonPID,
}

#[derive(Clone, Debug)]
pub struct PID(u64);

impl PID {
    #[inline]
    pub fn new(pid: u64) -> Self {
        PID(pid)
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum ProcessState {
    Running,
    Terminated,
    Waiting,
    NoThreads,
}

#[derive(Debug, Clone)]
pub struct Process {
    /// PID of the process, will be allocated linearly
    pub pid: PID,
    /// state - running/terminated or waiting
    pub state: ProcessState,
    /// Page table base physical address
    pub cr3: u64,
    /// thread IDs, points to thread IDs
    pub threads: Vec<ThreadID>,
    /// if true, the process is in userland
    pub user: bool,
    /// more fields will be added in future.
    pub name: String,

    /// root page table, if there are any
    pub pt_root: Option<Box<VirtualMemoryManager>>,
}

impl Process {
    #[inline]
    pub fn create_user_process(name: String) -> Self {
        // creates a new usermode process
        // clone the l4 page table of the kernel
        let k_vmm = KernelVirtualMemoryManager::pt();

        // allocate a new virtual address at 4k aligned region for new virtual address:
        let frame_opt = PhysicalMemoryManager::alloc();
        if frame_opt.is_none() {
            panic!("Failed to allocate memory for new Virtual page table. OOM");
        }

        let frame = frame_opt.unwrap();

        // get it's address:
        let new_pt_vaddr = VirtualAddress::from_u64(k_vmm.phy_offset + frame.as_u64());

        // clone the page table
        let page_table: &mut PageTable = unsafe { &mut *new_pt_vaddr.get_mut_ptr() };

        // copy the pages of kernel p4 table:
        let kernel_table: &mut PageTable = unsafe { &mut *k_vmm.l4_virtual_address.get_mut_ptr() };

        for idx in 0..kernel_table.entries.len() {
            page_table.entries[idx] = kernel_table.entries[idx].clone();
        }

        // create a new VMM object:
        let vmm = Box::new(VirtualMemoryManager {
            n_tables: 1,
            l4_virtual_address: new_pt_vaddr,
            l4_phy_addr: frame.addr(),
            phy_offset: k_vmm.phy_offset,
            offset_base_addr: k_vmm.l4_phy_addr,
        });

        let pid = new_pid();

        Process {
            pid,
            state: ProcessState::NoThreads,
            cr3: frame.addr().as_u64(),
            threads: Vec::new(),
            user: true,
            name,
            pt_root: Some(vmm),
        }
    }

    pub fn empty(name: String, user: bool) -> Self {
        if user {
            // create and return the user process:
            return Self::create_user_process(name);
        }

        // return the process:
        let kernel_cr3 = PhysicalAddress::from_u64(mmu::read_cr3());
        let pid = new_pid();

        log::debug!("Created empty kernel process, pid={}.", pid.as_u64());
        Process {
            pid,
            state: ProcessState::NoThreads,
            cr3: kernel_cr3.as_u64(),
            threads: Vec::new(),
            user,
            name,
            // kernel mode processes run with the same kernel page table.
            pt_root: None,
        }
    }

    #[inline]
    pub fn get_state(&self) -> ProcessState {
        self.state.clone()
    }

    #[inline]
    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    #[inline]
    pub fn is_usermode(&self) -> bool {
        self.user
    }

    #[inline]
    pub fn get_threads(&self) -> Vec<ThreadID> {
        self.threads.clone()
    }

    #[inline]
    pub fn get_page_table(&self) -> u64 {
        self.cr3
    }

    #[inline]
    pub fn get_thread_index(&self, th: &ThreadID) -> Option<usize> {
        for (idx, thread) in self.threads.iter().enumerate() {
            if thread.as_u64() == th.as_u64() {
                return Some(idx);
            }
        }

        None
    }

    #[inline]
    pub fn remove_thread(&mut self, thread_id: ThreadID) -> Result<(), ProcessError> {
        if let Some(index) = self.get_thread_index(&thread_id) {
            self.threads.remove(index);
            return Ok(());
        }
        Err(ProcessError::UnknownThreadID)
    }

    #[inline]
    pub fn add_thread(&mut self, thread_id: ThreadID) {
        log::debug!(
            "Adding thread {} for process {}:{}",
            thread_id.as_u64(),
            self.name,
            self.pid.as_u64()
        );
        self.threads.push(thread_id);
        // if process is in no thread state, make it to running:
        match self.state {
            ProcessState::NoThreads => self.state = ProcessState::Running,
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessPoolManager {
    pub user_proc_count: usize,
    pub kernel_proc_count: usize,
    pub pool_map: BTreeMap<u64, Process>,
}

impl ProcessPoolManager {
    pub fn init() -> Self {
        log::info!("Initializing Process Pool Manager.");
        ProcessPoolManager {
            user_proc_count: 0,
            kernel_proc_count: 0,
            pool_map: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn has_process(&self, pid: &PID) -> bool {
        self.pool_map.contains_key(&pid.as_u64())
    }

    #[inline]
    pub fn get_mut_ref(&mut self, pid: &PID) -> Option<&mut Process> {
        self.pool_map.get_mut(&pid.as_u64())
    }

    #[inline]
    pub fn get_ref(&mut self, pid: &PID) -> Option<&Process> {
        self.pool_map.get(&pid.as_u64())
    }

    pub fn debug_dump_pids(&self) {
        log::debug!("System processes:");
        for (pid, proc) in self.pool_map.iter() {
            log::debug!("pid={}, name={}", pid, proc.name);
        }
    }

    #[inline]
    pub fn remove_process(&mut self, pid: &PID) -> Result<(), ProcessError> {
        let res = self.pool_map.remove(&pid.as_u64());
        if res.is_none() {
            return Err(ProcessError::UnknwonPID);
        }

        if res.unwrap().is_usermode() {
            self.user_proc_count -= 1;
        } else {
            self.kernel_proc_count -= 1;
        }

        Ok(())
    }

    #[inline]
    pub fn add_process(&mut self, process: Process) {
        let pid = process.pid.as_u64();
        if process.is_usermode() {
            self.user_proc_count += 1;
        } else {
            self.kernel_proc_count += 1;
        }

        self.pool_map.insert(pid, process);
    }
}

lazy_static! {
    pub static ref PROCESS_POOL: Mutex<ProcessPoolManager> = Mutex::new(ProcessPoolManager::init());
}

pub fn setup_process_pool() {
    let pool_lock = PROCESS_POOL.lock();
    log::info!(
        "Process pool setup sucessfull. n_procs={}",
        pool_lock.kernel_proc_count + pool_lock.user_proc_count
    );
}

pub fn new(name: String, is_user: bool) -> PID {
    let process = Process::empty(name, is_user);
    let pid = process.pid.clone();
    PROCESS_POOL.lock().add_process(process);
    pid
}
