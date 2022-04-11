use crate::library;

use core::fmt;
use library::syscalls;
use library::types::Stdio;


pub struct SysStdout;

impl fmt::Write for SysStdout {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        unsafe {
            syscalls::sys_write(
                Stdio::Stdout as usize,
                string.as_bytes(),
                string.len()
            );
        }

        Ok(())
    }
}

pub fn write_stdout(data: &[u8], size: usize) -> usize {
    unsafe {
        syscalls::sys_write(
            Stdio::Stdout as usize,
            data,
            size
        )
    }
} 

pub fn read_stdin(buffer: &mut [u8], size: usize) -> usize {
    unsafe {
        syscalls::sys_read(
            Stdio::Stdin as usize,
            buffer,
            size
        )
    }
}
