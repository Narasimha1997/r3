#![no_std]


use core::panic::PanicInfo;

pub mod library;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[macro_export]
macro_rules! print {

    ($fmt:expr) => {
        let mut sys_stdout = library::utils::SysStdout{};
        let _ = sys_stdout.write_fmt(format_args!($fmt)).unwrap();
    };

    ($fmt:expr, $($arg:tt)*) => {
        let mut sys_stdout = library::utils::SysStdout{};
        let _ = sys_stdout.write_fmt(format_args!($fmt, $($arg)*)).unwrap();
    };
}

#[macro_export]
macro_rules! println {

    ($fmt:expr) => {
        let mut sys_stdout = library::utils::SysStdout{};
        let _ = sys_stdout.write_fmt(format_args!(
            concat!($fmt, "\n")
        )).unwrap();
    };

    ($fmt:expr, $($arg:tt)*) => (
        let mut sys_stdout = library::utils::SysStdout{};
        let _ = sys_stdout.write_fmt(format_args!(
            concat!($fmt, "\n"), $($arg)*
        ));
    );
}
