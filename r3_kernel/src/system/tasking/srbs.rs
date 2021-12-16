extern crate alloc;

use crate::alloc::boxed::Box;
use crate::cpu::state::CPURegistersState;
use crate::mm::VirtualAddress;
use crate::system::process::PID;
use crate::system::tasking::{Sched, ThreadSuspendType, ThreadWakeupType};
use crate::system::thread::{ContextType, Thread, ThreadID};

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct SleepingThread {
    pub till_ticks: usize,
    pub thread: Thread,
}

#[derive(Debug, Clone)]
pub struct WaitingThread {
    pub pid: PID,
    pub thread: Thread,
}

#[derive(Debug, Clone)]
pub struct WaitQueue {
    /// contains a list of threads that are waiting
    pub sleep_threads: Vec<SleepingThread>,
    pub waiting_threads: Vec<WaitingThread>,
}

impl WaitQueue {
    #[inline]
    pub fn empty() -> Self {
        WaitQueue {
            sleep_threads: Vec::new(),
            waiting_threads: Vec::new(),
        }
    }

    #[inline]
    pub fn put_sleep(&mut self, thread: Thread, ticks: usize) {
        self.sleep_threads.push(SleepingThread {
            till_ticks: ticks,
            thread,
        });
    }

    #[inline]
    pub fn put_wait(&mut self, thread: Thread, pid: PID) {
        self.waiting_threads.push(WaitingThread { pid, thread });
    }

    #[inline]
    pub fn dispatch_suspend(&mut self, thread: Thread, suspend_type: ThreadSuspendType) {
        match suspend_type {
            ThreadSuspendType::SuspendWait(pid) => {
                self.put_wait(thread, pid);
            }
            ThreadSuspendType::SuspendSleep(ticks) => {
                self.put_sleep(thread, ticks);
            }
            ThreadSuspendType::Nothing => {}
        }
    }

    #[inline]
    pub fn wake_waiting_threads(&mut self, pid: PID, run_queue: &mut Vec<Thread>) {
        self.waiting_threads = self
            .waiting_threads
            .drain_filter(|entry| {
                let should_retain = if entry.pid.as_u64() == pid.as_u64() {
                    run_queue.push(entry.thread.clone());
                    false
                } else {
                    true
                };

                should_retain
            })
            .collect();
    }

    #[inline]
    pub fn wake_sleeping_threads(&mut self, ticks: usize, run_queue: &mut Vec<Thread>) {
        // decrement the tick of each of thread
        // if ticks = 0 after decrementing, remove those threads and put them
        // into the run queue, this algorithm can be improved too much.
        // this is a rough implementation.
        self.sleep_threads = self
            .sleep_threads
            .drain_filter(|entry| {
                let should_retain = if entry.till_ticks - 1 == 0 {
                    // add this to run queue
                    run_queue.push(entry.thread.clone());
                    false
                } else {
                    entry.till_ticks = entry.till_ticks - ticks;
                    true
                };
                should_retain
            })
            .collect();
    }

    #[inline]
    pub fn dispatch_wakeup(&mut self, wakeup_mode: ThreadWakeupType, run_queue: &mut Vec<Thread>) {
        match wakeup_mode {
            ThreadWakeupType::FromSleep(ticks) => {
                self.wake_sleeping_threads(ticks, run_queue);
            }
            ThreadWakeupType::FromWait(pid) => {
                self.wake_waiting_threads(pid, run_queue);
            }
            ThreadWakeupType::Nothing => {}
        }
    }
}

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
        self.wait_queue.dispatch_wakeup(
            wakeup_mode,
            &mut self.thread_list
        );
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
