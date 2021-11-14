extern crate alloc;

use crate::cpu::state::CPURegistersState;
use crate::mm::{PhysicalAddress, VirtualAddress};
use crate::system::process::{PID, PROCESS_POOL};

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

static CURRENT_TID: AtomicU64 = AtomicU64::new(0);

pub fn new_tid() -> ThreadID {
    let current = CURRENT_TID.load(Ordering::SeqCst);
    if current + 1 == u64::max_value() {
        panic!("Could not create process, Out of PIDs");
    }

    CURRENT_TID.store(current + 1, Ordering::SeqCst);
    ThreadID(current)
}

#[derive(Clone, Debug)]
pub struct ThreadID(u64);

impl ThreadID {
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum ThreadState {
    Running,
    Waiting,
    // TODO: Define more states later.
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub parent_pid: PID,
    pub parent_cr3: PhysicalAddress,
    pub state: CPURegistersState,
    pub stack_end: VirtualAddress,
    pub name: String,
}




