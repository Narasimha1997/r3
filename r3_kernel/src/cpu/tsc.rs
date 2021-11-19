use core::sync::atomic::{AtomicU64, Ordering};

use crate::cpu::pit;

/// Represents the TSC ticks
#[derive(Clone, Copy)]
pub struct TSCTicks(u64);

impl TSCTicks {
    #[inline(always)]
    pub fn u64(&self) -> u64 {
        self.0
    }
}

static CPU_FREQUENCY: AtomicU64 = AtomicU64::new(0);
const TSC_RESET_ECX: u32 = 0x10;
const TSC_DEADLINE_ECX: u32 = 0x6e0;

pub enum SleepTimeRange {
    Seconds = 1000000000,
    MilliSeconds = 1000000,
    MicroSeconds = 1000,
}

impl SleepTimeRange {
    #[inline]
    pub fn get_range(ns: u64) -> Self {
        if ns > (Self::Seconds as u64) {
            Self::Seconds
        } else if ns > (Self::MilliSeconds as u64) {
            Self::MilliSeconds
        } else {
            Self::MicroSeconds
        }
    }
}

// some helpers
pub fn safe_ticks_from_ns(ns: u64) -> TSCTicks {
    let range = SleepTimeRange::get_range(ns);
    match range {
        SleepTimeRange::Seconds => {
            let ms = ns / 1000000;
            let frequency = TSC::read_cpu_frequency();
            TSCTicks((ms * frequency) / 1000)
        }
        SleepTimeRange::MilliSeconds => {
            let us = ns / 1000;
            let frequency = TSC::read_cpu_frequency();
            TSCTicks((us * frequency) / 1000000)
        }
        SleepTimeRange::MicroSeconds => {
            let frequency = TSC::read_cpu_frequency();
            TSCTicks((ns * frequency) * 1000000000)
        }
    }
}

pub fn ns_from_ticks(ticks: TSCTicks) -> u64 {
    let ticks_us = ticks.0 * 1000000;
    let current_frequency = TSC::read_cpu_frequency() / 1000;
    ticks_us / current_frequency
}

pub struct TSC;

impl TSC {
    #[inline]
    pub fn read_cpu_frequency() -> u64 {
        let value = CPU_FREQUENCY.load(Ordering::SeqCst);
        value
    }

    #[inline]
    pub fn read_tsc() -> TSCTicks {
        let rax: u64;
        let rdx: u64;

        unsafe {
            asm!(
                "rdtscp",
                out("rdx") rdx,
                out("rax") rax,
                out("rcx") _,
                options(nostack, nomem)
            )
        }

        TSCTicks(rdx << 32 | (rax & 0xffffffff))
    }

    #[inline]
    pub fn reset_tsc() {
        unsafe {
            asm!(
                "wrmsr",
                in("ecx") TSC_RESET_ECX,
                in("edx") 0 as u32,
                in("eax") 0 as u32,
                options(nomem, nostack)
            )
        }
    }

    pub fn detect_cpu_speed() {
        let t1 = TSC::read_tsc();
        pit::sleep_ns(10000000);
        let t2 = TSC::read_tsc();

        CPU_FREQUENCY.store(100 * (t2.0 - t1.0), Ordering::SeqCst);
    }
}

pub struct TSCTimerShot;

impl TSCTimerShot {
    pub fn set_shot_at(tick: TSCTicks) {
        let tick_high = (tick.0 >> 32) as u32;
        let tick_low = tick.0 as u32;
        unsafe {
            asm!(
                "wrmsr",
                in("ecx") TSC_DEADLINE_ECX,
                in("edx") tick_high,
                in("eax") tick_low,
                options(nomem, nostack)
            )
        }
    }

    pub fn reset_current_shot() {
        unsafe {
            asm!(
                "wrmsr",
                in("ecx") TSC_DEADLINE_ECX,
                in("edx") 0 as u32,
                in("eax") 0 as u32,
                options(nomem, nostack)
            )
        }
    }

    pub fn wait_for_shot_at(ticks: TSCTicks) {
        // reset any shot if pending
        Self::reset_current_shot();
        Self::set_shot_at(ticks.clone());

        unsafe {
            while TSC::read_tsc().0 < ticks.0 {
                asm!("sti; hlt;");
            }
            asm!("cli");
        }
    }

    pub fn create_shot_from_ns(ns: u64) {
        Self::set_shot_at(safe_ticks_from_ns(ns));
    }

    pub fn create_shot_after_ns(ns: u64) {
        let ticks = safe_ticks_from_ns(ns);
        Self::create_shot_from_ticks(ticks);
    }

    pub fn create_shot_from_ticks(ticks: TSCTicks) {
        let n_ticks = TSCTicks(TSC::read_tsc().0 + ticks.0);
        Self::set_shot_at(n_ticks);
    }
}

/// High level APIs to be used by caller functions
pub struct TSCSleeper;

impl TSCSleeper {
    pub fn sleep_ticks(ticks: u64) {
        let total_ticks = TSCTicks(TSC::read_tsc().0 + ticks);
        TSCTimerShot::wait_for_shot_at(total_ticks);
    }

    pub fn sleep_ns(ns: u64) {
        let ticks = safe_ticks_from_ns(ns);
        let total_ticks = TSCTicks(ticks.0 + TSC::read_tsc().0);
        TSCTimerShot::wait_for_shot_at(total_ticks);
    }

    pub fn sleep_us(us: u64) {
        let ns = us * 1000;
        Self::sleep_ns(ns);
    }

    pub fn sleep_ms(ms: u64) {
        let ns = ms * 1000000;
        Self::sleep_ns(ns);
    }

    pub fn sleep_sec(sec: u64) {
        let ns = sec * 1000000000;
        Self::sleep_ns(ns);
    }
}

pub fn init_timer() {
    log::info!("Enabling CPU timestamp counter..");
    TSC::detect_cpu_speed();
    let cpu_frequency = TSC::read_cpu_frequency();
    log::info!("Enabled CPU TSC, cpu_frequency={}", cpu_frequency);
}
