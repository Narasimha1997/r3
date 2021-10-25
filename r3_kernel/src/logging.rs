// provides macros for basic logging with log levels
extern crate log;

use core::fmt::Write;
use core::panic::PanicInfo;

use log::{LevelFilter, Metadata, Record};

use crate::drivers::uart;
use uart::UART_DRIVER;

// a logger that implements kernel logging functionalities
pub struct KernelLogger;

// implement a writer trait for UART_DRIVER

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

impl log::Log for KernelLogger {
    fn enabled(&self, _meta: &Metadata) -> bool {
        // TOOD: Add level based filtering
        true
    }

    fn log(&self, record: &Record) {
        // TODO: Add level wise filtering and support multiple channels
        print_uart!(
            "{:20} {:5} {}",
            record.target(),
            record.level(),
            record.args()
        );
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
