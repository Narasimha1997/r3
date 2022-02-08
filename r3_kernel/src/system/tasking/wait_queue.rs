extern crate alloc;

use crate::system::process::PID;
use crate::system::tasking::{ThreadSuspendType, ThreadWakeupType};
use crate::system::thread::Thread;

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
