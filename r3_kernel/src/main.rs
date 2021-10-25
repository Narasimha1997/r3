#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

#![feature(asm)] // enable asm

extern crate bootloader;
extern crate log;

pub mod cpu;
pub mod drivers;
pub mod logging;
pub mod boot_proto;

use core::panic::PanicInfo;
use bootloader::BootInfo;
use boot_proto::BootProtocol;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {

    // init basic logging through UART as of now:
    logging::init();
    BootProtocol::create(boot_info);
    log::info!("Saving boot info");

    loop {}
}

pub unsafe fn exit_qemu() {
   
}