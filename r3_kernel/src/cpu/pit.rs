extern crate log;
extern crate spin;

use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

use crate::cpu::io::Port;
use lazy_static::lazy_static;

/// PIT interrupt channel 0
const PIT_CMD_CHANNEL_0: usize = 0x40;

/// PIT interrupt channel 2 (channel-1 is not required for us.)
const PIT_CMD_CHANNEL_2: usize = 0x42;

/// PIT register used to deal with commands.
const PIT_COMMAND_REGISTER: usize = 0x43;

/// By default, the PIT hardware produces frequencies at 1.19 Mhz
const PIT_OSCILLATION_FREQUENCY: u32 = 1193182;

/// this is the minimum frequency PIT can operate.
const PIT_LEAST_FREQUENCY: u32 = 19;

/// Has all the methods required to handle PIT commands,
/// This will always be handled with lock
pub struct PITCommandControl {
    pub channel_0: Mutex<Port>,
    pub channel_1: Mutex<Port>,
    pub command: Mutex<Port>,
}

impl PITCommandControl {
    #[inline]
    pub fn write_channel_0(&self, value: u8) {
        self.channel_0.lock().write_u8(value);
    }

    #[inline]
    pub fn write_channel_1(&self, value: u8) {
        self.channel_1.lock().write_u8(value);
    }

    #[inline]
    pub fn write_command(&self, value: u8) {
        self.command.lock().write_u8(value);
    }

    pub fn new() -> PITCommandControl {
        PITCommandControl {
            channel_0: Mutex::new(Port::new(PIT_CMD_CHANNEL_0, false)),
            channel_1: Mutex::new(Port::new(PIT_CMD_CHANNEL_2, false)),
            command: Mutex::new(Port::new(PIT_COMMAND_REGISTER, false)),
        }
    }
}

pub fn init() -> PITCommandControl {
    let pit = PITCommandControl::new();
    pit
}

lazy_static! {
    pub static ref PIT: PITCommandControl = init();
}

static PIT_TICKS: AtomicUsize = AtomicUsize::new(0);

fn setup_timer(frequency: u32) -> u64 {
    let pit: &PITCommandControl = &PIT;
    let div = PIT_OSCILLATION_FREQUENCY / frequency;

    if div > (u16::max_value() as u32) {
        panic!(
            "The choosen PIT frequency should be atleast > {}",
            PIT_LEAST_FREQUENCY
        );
    }

    pit.write_command(0b00_11_010_0);
    pit.write_channel_0(div as u8);
    pit.write_channel_0((div >> 8) as u8);

    let running_frequency = PIT_OSCILLATION_FREQUENCY / div;

    // 1 ns
    return 1000000000 / (running_frequency as u64);
}

#[inline]
pub fn pit_callback() {
    PIT_TICKS.fetch_add(1, Ordering::SeqCst);
}

pub fn sleep_ns(ns: u64) {
    PIT_TICKS.store(0, Ordering::SeqCst);

    // set with 1000hz initially
    let ns_in_tick = setup_timer(1000);

    let n_ticks = (ns / ns_in_tick) as usize;

    log::debug!("n_ticks: {}", n_ticks);

    unsafe {
        asm!("sti");
        while PIT_TICKS.load(Ordering::SeqCst) < n_ticks {
            asm!("hlt");
        }

        asm!("cli");
    }
}

pub fn reset_timer() {
    setup_timer(PIT_LEAST_FREQUENCY);
}