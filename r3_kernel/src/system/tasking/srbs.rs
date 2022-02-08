extern crate alloc;

use crate::alloc::boxed::Box;
use crate::cpu::state::CPURegistersState;
use crate::mm::VirtualAddress;
use crate::system::process::PID;
use crate::system::tasking::wait_queue::WaitQueue;
use crate::system::tasking::{Sched, ThreadSuspendType, ThreadWakeupType};
use crate::system::thread::{ContextType, Thread, ThreadID};

use alloc::vec::Vec;

#[derive(Debug, Clone)]
/// A scheduler that schedules tasks from
/// thread queue. As of now, this scheduler is not
/// actually multiprocessor, and it schedules only for BSP.
/// the name is in par with future ideas. This is based on a simple round
/// robin approach, with simple semantics.
pub struct SimpleRoundRobinSchduler {
    pub thread_list: Vec<Thread>,
    pub thread_index: Option<usize>,
    pub wait_queue: WaitQueue,
    pub suspend_next: bool,
    pub suspend_type: ThreadSuspendType,
}

impl Sched for SimpleRoundRobinSchduler {
    fn empty() -> Self {
        SimpleRoundRobinSchduler {
            thread_list: Vec::new(),
            thread_index: None,
            wait_queue: WaitQueue::empty(),
            suspend_next: false,
            suspend_type: ThreadSuspendType::Nothing,
        }
    }

    /// Adds a new function to the list
    fn add_new_thread(&mut self, thread: Thread) {
        log::debug!("Adding thread {:?} to the thread queue.", thread.thread_id);
        self.thread_list.push(thread);
    }

    fn save_current_ctx(&mut self, state: CPURegistersState) {
        if let Some(thread_id) = self.thread_index {
            if let Some(thread_ref) = self.thread_list.get_mut(thread_id) {
                thread_ref.context = Box::new(ContextType::SavedContext(state));
            }
        }

        if self.suspend_next {
            // suspend this thread
            let thread_idx = self.thread_index.unwrap();
            let thread = self.thread_list.remove(thread_idx);
            self.wait_queue
                .dispatch_suspend(thread, self.suspend_type.clone());
            self.thread_index = None;
            self.suspend_next = false;
            self.suspend_type = ThreadSuspendType::Nothing;
        }
    }

    fn exit(&mut self, code: i64) {
        // initiate exit operation:
        // 1. get the thread index
        if let Some(thread_index) = self.thread_index {
            // remove the thread from the queue
            // get the thread ID
            let thread_ref = self.thread_list.get_mut(thread_index).unwrap();
            thread_ref.exit();
            log::debug!(
                "Thread {} exited with code={}",
                thread_ref.thread_id.as_u64(),
                code
            );
            // remove the thread
            self.thread_list.remove(thread_index);
            self.thread_index = None;
        }
    }

    fn lease_next_thread(&mut self) -> Option<Thread> {
        // got a schedule request
        if self.thread_list.is_empty() {
            return None;
        }

        // we have a thread
        let thread_ref_opt = {
            let n_threads = self.thread_list.len();
            let thread_idx_ref = self.thread_index.get_or_insert(0);
            // round robin
            let next_thread_idx = (*thread_idx_ref + 1) % n_threads;
            *thread_idx_ref = next_thread_idx;

            self.thread_list.get_mut(next_thread_idx)
        };

        if let Some(thread_ref) = thread_ref_opt {
            thread_ref.sched_count += 1;
            return Some(thread_ref.clone());
        };

        None
    }

    fn current_tid(&self) -> Option<ThreadID> {
        if self.thread_index.is_none() {
            return None;
        }

        let thread = self.thread_list.get(self.thread_index.unwrap());
        Some(thread.as_ref().unwrap().thread_id)
    }

    fn current_pid(&self) -> Option<PID> {
        if self.thread_index.is_none() {
            return None;
        }

        let thread = self.thread_list.get(self.thread_index.unwrap());
        Some(thread.as_ref().unwrap().parent_pid.clone())
    }

    fn check_wakeup(&mut self, wakeup_mode: ThreadWakeupType) {
        self.wait_queue
            .dispatch_wakeup(wakeup_mode, &mut self.thread_list);
    }

    fn suspend_thread(&mut self, suspend_type: ThreadSuspendType) {
        if self.thread_index.is_none() {
            // no threads running currently
            return;
        }

        self.suspend_next = true;
        self.suspend_type = suspend_type;
    }

    fn reset_current_thread_stack(&mut self) -> VirtualAddress {
        if let Some(thread_idx) = self.thread_index {
            let thread_ref: &mut Thread = self.thread_list.get_mut(thread_idx).unwrap();
            return thread_ref.reset_stack();
        }

        VirtualAddress::from_u64(0)
    }
}
