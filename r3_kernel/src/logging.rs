// provides macros for basic logging with log levels
extern crate log;

use core::fmt::Write;
use core::panic::PanicInfo;

use log::{Level, LevelFilter, Metadata, Record};

use crate::drivers::{display::fb_text::FRAMEBUFFER_LOGGER, display::framebuffer::Pixel, uart};
use uart::UART_DRIVER;

// a logger that implements kernel logging functionalities
pub struct KernelLogger;

fn get_color(level: Level) -> Pixel {
    match level {
        Level::Error => Pixel {
            b: 0,
            g: 0,
            r: 255,
            channel: 0,
        },
        Level::Warn => Pixel {
            b: 0,
            g: 255,
            r: 255,
            channel: 0,
        },
        _ => Pixel {
            b: 255,
            g: 255,
            r: 255,
            channel: 0,
        },
    }
}

// a macro that takes care of writing string to UART:
macro_rules! print_uart {
    ($fmt:expr, $($arg:tt)*) => (
        if UART_DRIVER.is_some() {
            let mutexed_uart = UART_DRIVER.as_ref().unwrap();
            mutexed_uart.lock().write_fmt(
                format_args!(concat!($fmt, "\n"), $($arg)*)
            ).unwrap();
        }
    );
}

macro_rules! print_framebuffer {
    ($level:expr, $fmt:expr, $($arg:tt)*) => {
        let mut fb_lgr_lock = FRAMEBUFFER_LOGGER.lock();
        fb_lgr_lock.set_color(get_color($level));
        let _ = fb_lgr_lock.write_fmt(
            format_args!(concat!($fmt, "\n"), $($arg)*)
        );
    };
}

impl log::Log for KernelLogger {
    fn enabled(&self, _meta: &Metadata) -> bool {
        // TOOD: Add level based filtering
        true
    }

    fn log(&self, record: &Record) {
        let level = record.level();

        if level <= LevelFilter::Trace {
            print_uart!(
                "{:20} {:5} {}",
                record.target(),
                record.level(),
                record.args()
            );
        }

        if level <= LevelFilter::Info {
            print_framebuffer!(
                level,
                "{:20} {:5} {}",
                record.target(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        // TODO: Will be used in future for dmesg
    }
}

static KERNEL_LOGGER: KernelLogger = KernelLogger;

pub fn init() {
    // unuse the result
    let _ = log::set_logger(&KERNEL_LOGGER);
    log::set_max_level(LevelFilter::Debug);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // write the panic info and loop infinitely:
    log::error!("{}", info);
    loop {}
}
