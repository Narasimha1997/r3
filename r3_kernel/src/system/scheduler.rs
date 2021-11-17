extern crate alloc;
extern crate log;
extern crate spin;

use alloc::vec::Vec;

use crate::cpu::state::CPURegistersState;
use crate::system::thread::{ContextType, Thread};
use spin::Mutex;

use lazy_static::lazy_static;

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

impl SimpleRoundRobinSchduler {
    pub fn empty() -> Self {
        SimpleRoundRobinSchduler {
            thread_list: Vec::new(),
            thread_index: None,
        }
    }

    /// Adds a new function to the list
    pub fn add_new_thread(&mut self, thread: Thread) {
        log::debug!("Adding thread {:?} to the thread queue.", thread.thread_id);
        self.thread_list.push(thread);
    }

    pub fn save_current_ctx(&mut self, state: CPURegistersState) {
        if let Some(thread_id) = self.thread_index {
            if let Some(thread_ref) = self.thread_list.get_mut(thread_id) {
                thread_ref.context = ContextType::SavedContext(state);
            }
        }
    }

    pub fn run_schedule(&mut self, state: CPURegistersState) -> Option<Thread> {
        self.save_current_ctx(state);
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

lazy_static! {
    pub static ref SCHEDULER: Mutex<SimpleRoundRobinSchduler> =
        Mutex::new(SimpleRoundRobinSchduler::empty());
}

pub fn setup_scheduler() {
    log::info!(
        "Setup scheduler successful, initial thread={:?}",
        SCHEDULER.lock().thread_index
    );
}
