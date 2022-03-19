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

    let fstat_info_res = FILESYSTEM.lock().fstat(&mut fd);

    if fstat_info_res.is_err() {
        return Err(LoadError::FileReadError);
    }

    let fstat_info = fstat_info_res.unwrap();

    // allocate a temp buffer to read one block at a time
    let mut temp_buffer: Vec<u8> = Vec::new();
    temp_buffer.resize(fstat_info.block_size, 0);

    // data will be copied to this buffer
    let mut binary_buffer: Vec<u8> = Vec::new();
    binary_buffer.resize(fstat_info.file_size, 0);

    // log::debug!("FSTAT: {} {} {}", fstat_info.file_size, fstat_info.blocks, fstat_info.block_size);

    for idx in 0..fstat_info.blocks {
        let read_res = FILESYSTEM.lock().read(&mut fd, &mut temp_buffer);
        if read_res.is_err() {
            log::debug!("ELF load failed, {:?}", read_res.unwrap_err());
            return Err(LoadError::FileReadError);
        }

        let n_read = read_res.unwrap();
        let slice_ref: &mut [u8] = binary_buffer.as_mut();

        let current_start = idx * fstat_info.block_size;
        slice_ref[current_start..current_start + n_read].copy_from_slice(&temp_buffer[0..n_read]);

        if n_read < 512 {
            break;
        }

        let seek_result = FILESYSTEM.lock().seek(&mut fd, 512, SeekType::SEEK_CUR);
        if seek_result.is_err() {
            break;
        }
    }

    Ok(binary_buffer)
}
