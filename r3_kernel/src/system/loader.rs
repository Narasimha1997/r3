extern crate alloc;
extern crate object;

use crate::mm::paging::VirtualMemoryManager;
use crate::mm::VirtualAddress;
use crate::system::filesystem::vfs::FILESYSTEM;

use crate::system::filesystem::{FDOps, FSOps};

use alloc::vec::Vec;
use core::str;

#[derive(Debug, Clone)]
pub enum LoaderError {
    InvalidELF,
    InvalidFormat,
    FileReadError,
}

pub struct ELFLoader;

impl ELFLoader {
    #[inline]
    pub fn is_elf(binary: &[u8]) -> bool {
        if binary.len() < 4 {
            return false;
        }

        unsafe { str::from_utf8_unchecked(&binary[0..4]) == "ELF" }
    }

    pub fn elf_from_file(path: &str) -> Result<Vec<u8>, LoaderError> {
        // open the path
        let fd_res = FILESYSTEM.lock().open(path, 0);
        if fd_res.is_err() {
            log::debug!("ELF load failed, {:?}", fd_res.unwrap_err());
            return Err(LoaderError::FileReadError);
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
                return Err(LoaderError::FileReadError);
            }

            let n_read = read_res.unwrap();

            if iter == 0 {
                // check if it's a valid ELF
                if !Self::is_elf(&temp_buffer) {
                    log::debug!("ELF load failed, {} is not an ELF binary.", path);
                    return Err(LoaderError::InvalidFormat);
                }
            }

            binary_buffer.extend_from_slice(&temp_buffer[0..n_read]);

            if n_read < 512 {
                break;
            }

            let seek_result = FILESYSTEM.lock().seek(&mut fd, (iter + 1) * 512);
            if seek_result.is_err() {
                log::debug!("ELF load failed, {:?}", seek_result.unwrap_err());
                return Err(LoaderError::FileReadError);
            }

            iter += 1;
        }

        Ok(binary_buffer)
    }

    pub fn load_from_path(
        path: &str,
        vmm: &mut VirtualMemoryManager,
        at_addr: VirtualAddress,
    ) -> Result<VirtualAddress, LoaderError> {
        return Err(LoaderError::InvalidELF);
    }
}
