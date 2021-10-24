#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

#![feature(asm)] // enable asm

pub mod cpu;

use core::panic::PanicInfo;
use cpu::io::Port;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {

    // print some characters on the screen and loop
    let text = "Hello, World!!!";

    let io_port = Port::new(0x3f8, false);

    for ch in text.as_bytes().iter() {
        io_port.write_u8(*ch);
    }

    loop {}
}

pub unsafe fn exit_qemu() {
   
}