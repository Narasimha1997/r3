extern crate alloc;

use crate::system::filesystem::vfs::FILESYSTEM;

use crate::system::filesystem::{FDOps, FSOps, SeekType};

use alloc::vec::Vec;
use core::str;

#[derive(Debug, Clone)]
pub enum LoadError {
    InvalidFormat,
    FileReadError,
}

pub fn is_elf(binary: &[u8]) -> bool {
    if binary.len() < 4 {
        return false;
    }

    unsafe { str::from_utf8_unchecked(&binary[1..4]) == "ELF" }
}

pub fn read_executable(path: &str) -> Result<Vec<u8>, LoadError> {
    // open the path
    let fd_res = FILESYSTEM.lock().open(path, 0);
    if fd_res.is_err() {
        log::debug!("ELF load failed, {:?}", fd_res.unwrap_err());
        return Err(LoadError::FileReadError);
    }

    let mut fd = fd_res.unwrap();

    // read this until we hit the end
    let mut temp_buffer: Vec<u8> = Vec::new();
    temp_buffer.resize(512, 0);

    let mut binary_buffer: Vec<u8> = Vec::new();

    let mut iter = 0;

    loop {
        let read_res = FILESYSTEM.lock().read(&mut fd, &mut temp_buffer);
        if read_res.is_err() {
            log::debug!("ELF load failed, {:?}", read_res.unwrap_err());
            return Err(LoadError::FileReadError);
        }

        let n_read = read_res.unwrap();
        binary_buffer.extend_from_slice(&temp_buffer[0..n_read]);

        if n_read < 512 {
            break;
        }

        log::debug!("Loading {}", iter);
        iter +=1;

        let seek_result = FILESYSTEM.lock().seek(&mut fd, 512, SeekType::SEEK_CUR);
        if seek_result.is_err() {
            break;
        }
    }

    Ok(binary_buffer)
}
