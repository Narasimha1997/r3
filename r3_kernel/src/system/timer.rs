extern crate log;
extern crate spin;

use crate::cpu::tsc::{safe_ticks_from_ns, TSCTimerShot, TSC};
use spin::Mutex;

#[derive(Debug)]
#[repr(u64)]
pub enum Time {
    Second = 1000000000,
    MilliSecond = 1000000,
    MicroSecond = 1000,
    NanoSecond = 1,
}

/// each tick contains these many time nanoseconds.
const SYSTEM_TICK_DURATION: u64 = 10 * 1000000;

/// SystemTicker that keeps tracks of number of
/// ticks and provides few functions to manage timer.
pub struct SystemTicker {
    ticks: u64,
    epochs: u64,
}

impl SystemTicker {
    #[inline]
    pub fn reset(&mut self) {
        self.ticks = 0;
        self.epochs = 0;
    }

    #[inline]
    pub fn ticks_in_epoch(&self) -> u64 {
        self.ticks
    }

    #[inline]
    pub fn total_ticks(&self) -> u128 {
        (self.epochs * u64::max_value() + self.ticks) as u128
    }

    #[inline]
    pub fn update_tick(&mut self) {
        if self.ticks >= u64::max_value() {
            self.epochs += 1;
            self.ticks = 0;
        }

        self.ticks += 1;
    }

    #[inline]
    pub fn as_ns(&mut self) -> u128 {
        self.total_ticks() * SYSTEM_TICK_DURATION as u128
    }

    #[inline]
    pub const fn empty() -> Self {
        SystemTicker {
            ticks: 0,
            epochs: 0,
        }
    }
}

static SYSTEM_TICKS: Mutex<SystemTicker> = Mutex::new(SystemTicker::empty());

/// Provides methods to control timer
pub struct SystemTimer;

impl SystemTimer {
    #[inline]
    pub fn next_shot() {
        TSCTimerShot::reset_current_shot();
        TSCTimerShot::create_shot_after_ns(SYSTEM_TICK_DURATION);
    }

    /// This function will be called after every timer show
    #[inline]
    pub fn post_shot() {
        let mut ticks_lock = SYSTEM_TICKS.lock();
        ticks_lock.update_tick();
        Self::next_shot();
    }

    #[inline]
    pub fn manual_shot() {
        TSCTimerShot::reset_current_shot();
        // creates a manual time shot:
        unsafe {
            // call an interrupt over line 48
            // i.e 32 + 16, which is the tsc deadline interrupt line.
            asm!("0x30");
        }
    }

    #[inline]
    pub fn start_ticks() {
        Self::next_shot();
    }
}

#[inline]
/// spin loop for x nanoseconds,
/// this is different from sleep, as this will use spin loop
/// instead of actual sleep.
pub fn wait_ns(ns: u64) {
    let current = TSC::read_tsc();
    let offset = safe_ticks_from_ns(ns);

    while (TSC::read_tsc().u64() - current.u64()) < offset.u64() {
        for _ in 0..100 {}
    }
}
