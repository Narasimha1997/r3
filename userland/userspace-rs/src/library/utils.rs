use crate::library;

use core::{fmt, str};
use library::syscalls;
use library::types::{Stdio, UTSName, FStatInfo};

pub struct SysStdout;

impl fmt::Write for SysStdout {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        unsafe {
            syscalls::sys_write(Stdio::Stdout as usize, string.as_bytes(), string.len());
        }

        Ok(())
    }
}

pub fn write_stdout(data: &[u8], size: usize) -> usize {
    unsafe { syscalls::sys_write(Stdio::Stdout as usize, data, size) }
}

pub fn read_stdin(buffer: &mut [u8], size: usize) -> usize {
    unsafe { syscalls::sys_read(Stdio::Stdin as usize, buffer, size) }
}

pub fn get_uname() -> Result<UTSName, usize> {
    let mut uts_name = UTSName::empty();
    let result = unsafe { syscalls::sys_uname(&mut uts_name) };
    if result == 0 {
        return Ok(uts_name);
    }

    Err(result)
}

pub fn power_off_machine() {
    unsafe {
        syscalls::sys_shutdown();
    }
}

pub fn lstat(path: &str) -> Result<FStatInfo, usize> {
    let mut stat = FStatInfo::default();
    let result = unsafe { syscalls::sys_lstat(path, &mut stat) };
    if result == 0 {
        return Ok(stat);
    }

    Err(result)
}

pub unsafe fn str_from_c_like_buffer(utf8_src: &[u8]) -> &str {
    let mut nul_range_end = 1_usize;
    for b in utf8_src {
        if *b == 0 {
            break;
        }
        nul_range_end += 1;
    }
    return str::from_utf8_unchecked(&utf8_src[0..nul_range_end]);
}
