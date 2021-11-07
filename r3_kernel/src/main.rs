#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)]
#![feature(asm)] // enable asm
#![feature(alloc_error_handler)] // enable allocation errors

extern crate bootloader;
extern crate log;

pub mod cpu;
pub mod drivers;
pub mod logging;
pub mod boot_proto;
pub mod mm;

use bootloader::BootInfo;
use boot_proto::BootProtocol;

/// This function is called on panic.


#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {

    // init basic logging through UART as of now:
    logging::init();
    BootProtocol::create(boot_info);
    log::info!("Saving boot info");
    log::info!("Hello, kernel world!");

    BootProtocol::print_boot_info();

    cpu::init_features_detection();
    cpu::init_base_processor_tables();

    cpu::run_test_breakpoint_recovery();

    mm::init();

    loop {}
}