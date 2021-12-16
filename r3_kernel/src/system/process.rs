extern crate alloc;
extern crate log;
extern crate spin;

use crate::cpu::mmu;
use crate::mm::paging::{KernelVirtualMemoryManager, VirtualMemoryManager};
use crate::mm::{PhysicalAddress, VirtualAddress};
use crate::system::thread::ThreadID;
use crate::system::utils::{
    create_cloned_layout, create_default_descriptors, create_process_layout, reset_layout,
    ProcessData,
};

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
    /// parent PID
    pub ppid: PID,
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
    /// process data, this will be null for kernel process
    pub proc_data: Option<ProcessData>,
}

impl Process {
    #[inline]
    pub fn create_from_parent(ppid: &PID) -> Self {
        let (mut vmm, frame_addr) = KernelVirtualMemoryManager::new_vmm();
        let pid = new_pid();

        let mut parent_lock = PROCESS_POOL.lock();
        let parent_opt = parent_lock.get_mut_ref(&ppid);

        if parent_opt.is_none() {
            panic!("PID {} not found.", ppid.as_u64());
        }

        let parent = parent_opt.unwrap();

        let proc_data = Some(create_cloned_layout(
            &mut parent.proc_data.as_mut().unwrap(),
            &mut parent.pt_root.as_mut().unwrap(),
            &mut vmm,
        ));

        Process {
            pid,
            ppid: parent.pid.clone(), // as of now
            state: ProcessState::NoThreads,
            cr3: frame_addr.as_u64(),
            threads: Vec::new(),
            user: true,
            name: parent.name.clone(),
            pt_root: Some(Box::new(vmm)),
            proc_data,
        }
    }

    #[inline]
    pub fn create_user_process(name: String, path: &str) -> Self {
        let (mut vmm, frame_addr) = KernelVirtualMemoryManager::new_vmm();
        let pid = new_pid();

        let proc_data = if path.len() > 0 {
            let mut p_data = create_process_layout(path, &mut vmm);
            create_default_descriptors(&mut p_data);
            Some(p_data)
        } else {
            None
        };

        Process {
            pid,
            ppid: PID(0), // as of now
            state: ProcessState::NoThreads,
            cr3: frame_addr.as_u64(),
            threads: Vec::new(),
            user: true,
            name,
            pt_root: Some(Box::new(vmm)),
            proc_data,
        }
    }

    pub fn empty(name: String, user: bool, path: &str) -> Self {
        if user {
            // create and return the user process:
            return Self::create_user_process(name, &path);
        }

        // return the process:
        let kernel_cr3 = PhysicalAddress::from_u64(mmu::read_cr3());
        let pid = new_pid();

        Process {
            pid,
            ppid: PID(0), // as of now,
            state: ProcessState::NoThreads,
            cr3: kernel_cr3.as_u64(),
            threads: Vec::new(),
            user,
            name,
            pt_root: None,
            proc_data: None,
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

    #[inline]
    pub fn exit(&mut self, _code: usize) {
        log::debug!("Exiting process {}", self.pid.as_u64());
        // TODO: unmap all the memory, because as of now
        // physical memory is not freed.
        if !self.is_usermode() {
            return;
        }

        reset_layout(
            "",
            &mut self.proc_data.as_mut().unwrap(),
            &mut self.pt_root.as_mut().unwrap(),
            false,
        );
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

    #[inline]
    pub fn reset_process(&mut self, pid: &PID, path: &str) -> VirtualAddress {
        let process_mut: &mut Process = self.pool_map.get_mut(&pid.as_u64()).unwrap();

        // reset the internal layout:
        reset_layout(
            path,
            &mut process_mut.proc_data.as_mut().unwrap(),
            &mut process_mut.pt_root.as_mut().unwrap(),
            true,
        )
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

pub fn new(name: String, is_user: bool, path: &str) -> PID {
    let process = Process::empty(name, is_user, &path);
    let pid = process.pid.clone();
    PROCESS_POOL.lock().add_process(process);
    pid
}
