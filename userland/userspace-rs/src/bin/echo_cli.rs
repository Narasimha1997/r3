#![no_std]
#![no_main]

use core::fmt::Write;
use userspace_rs::library;
use core::str;

use library::utils::read_stdin;
use userspace_rs::{println, print};

#[no_mangle]
pub extern "C" fn _start() {
    let mut data_buffer: [u8; 1024] = [0; 1024];
   
        println!("Hey, type something");

        loop {
            print!(">>> ");
            let read_length = read_stdin(&mut data_buffer, 1024);
            
            let utf_8_res = str::from_utf8(&data_buffer[0..read_length]);
            if utf_8_res.is_err() {
                println!("invalid string");
            } else {
                println!("you typed: {}", utf_8_res.unwrap());
            }

            for idx in 0..read_length {
                data_buffer[idx] = 0;
            }
        }
}