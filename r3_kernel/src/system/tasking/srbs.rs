use alloc::vec::Vec;

use crate::cpu::state::CPURegistersState;
use crate::system::tasking::{handle_exit, Sched};
use crate::system::thread::{ContextType, Thread};

#[derive(Debug, Clone)]
/// A scheduler that schedules tasks from
/// thread queue. As of now, this scheduler is not
/// actually multiprocessor, and it schedules only for BSP.
/// the name is in par with future ideas. This is based on a simple round
/// robin approach, with simple semantics.
pub struct SimpleRoundRobinSchduler {
    pub thread_list: Vec<Thread>,
    pub thread_index: Option<usize>,
}

impl Sched for SimpleRoundRobinSchduler {
    fn empty() -> Self {
        SimpleRoundRobinSchduler {
            thread_list: Vec::new(),
            thread_index: None,
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
                thread_ref.context = ContextType::SavedContext(state);
            }
        }
    }

    fn exit(&mut self, code: u64) {
        // initiate exit operation:
        // 1. get the thread index
        if let Some(thread_index) = self.thread_index {
            // remove the thread from the queue
            // get the thread ID
            let thread_ref = self.thread_list.get(thread_index).unwrap();
            handle_exit(&thread_ref);
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
}
