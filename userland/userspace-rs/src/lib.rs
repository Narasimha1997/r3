#![no_std]
#![feature(asm)]

use core::panic::PanicInfo;

pub mod library;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
