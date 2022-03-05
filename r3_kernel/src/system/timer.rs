extern crate log;
extern crate spin;

use crate::cpu::tsc::{safe_ticks_from_ns, TSCTimerShot, TSC};
use crate::mm::Alignment;
use crate::system::abi;
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
const SYSTEM_TICK_DURATION: u64 = 15 * 1000000;

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
    pub fn total_ticks(&self) -> u64 {
        (self.epochs * u64::max_value() + self.ticks) as u64
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
    pub fn as_ns(&mut self) -> u64 {
        self.total_ticks() * SYSTEM_TICK_DURATION as u64
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

    #[inline]
    pub fn enable_shot() {
        TSCTimerShot::create_shot_from_ns(SYSTEM_TICK_DURATION);
    }

    #[inline]
    pub fn disable_shots() {
        TSCTimerShot::reset_current_shot();
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
            // i.e 0x50, which is the tsc deadline interrupt line.
            asm!("int 0x50");
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

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct PosixTimeval {
    pub tv_sec: abi::CTime,
    pub tv_usec: abi::CSubSeconds,
}

impl PosixTimeval {
    pub fn from_ticks() -> Self {
        let ns = SYSTEM_TICKS.lock().as_ns();
        let seconds = ns as i64 / Time::Second as i64;

        // get microseconds offset
        let offset = ns as i64 - (seconds * Time::NanoSecond as i64);
        let offset_us = offset / Time::MicroSecond as i64;

        PosixTimeval {
            tv_sec: seconds,
            tv_usec: offset_us,
        }
    }

    #[inline]
    pub fn empty() -> Self {
        PosixTimeval {
            tv_sec: 0,
            tv_usec: 0,
        }
    }

    #[inline]
    pub fn mills(&self) -> u64 {
        (self.tv_sec as u64 * 1000) + (self.tv_usec as u64 / 1000)
    }

    #[inline]
    pub fn to_ticks(&self) -> usize {
        let mills = self.mills();
        let ns = mills * 1000000;
        (Alignment::align_up(ns, SYSTEM_TICK_DURATION) / SYSTEM_TICK_DURATION) as usize
    }
}

/// this will disable timer ticks and interrupts
pub fn pause_events() {
    TSCTimerShot::reset_current_shot();
}

/// this will enable timer ticks and interrupts
pub fn resume_events() {
    TSCTimerShot::reset_current_shot();
    TSCTimerShot::create_shot_after_ns(SYSTEM_TICK_DURATION);
}
