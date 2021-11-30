extern crate alloc;
extern crate log;

use crate::system::filesystem::devfs::DevFSDriver;
use crate::system::filesystem::{FDOps, FSOps};
use crate::system::filesystem::{FSError, FileDescriptor};

use alloc::{format, string::String};
use core::mem;
use core::str;

#[derive(Debug, Clone)]
#[repr(C, packed)]
struct TarHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    checksum: [u8; 8],
    f_type: u8,
    linked_name: [u8; 100],
    signature: [u8; 6],
    version: [u8; 2],
    usr_name: [u8; 32],
    group_name: [u8; 32],
    dev_major: [u8; 8],
    dev_minor: [u8; 8],
    name_prefix: [u8; 155],
    reserved: [u8; 12],
}

const HEADER_SIZE: usize = mem::size_of::<TarHeader>();

pub struct TarFS;

#[derive(Debug, Clone)]
pub struct TarFileDescriptor {
    /// offset of the file on that device
    pub offset: usize,
    /// size of the file
    pub size: usize,
    /// open flags
    pub flags: u32,
    ///seeked offset
    pub seeked_offset: usize,
}

#[inline]
fn oct_to_usize(buffer: &[u8]) -> usize {
    let mut multiplier = 1;
    let mut number = 0;
    let last_index = buffer.len() - 1;

    for idx in 0..(last_index + 1) {
        let byte = buffer[last_index - idx];
        if byte as char >= '0' && byte as char <= '7' {
            number += ((byte - 48) as usize) * multiplier;
            multiplier *= 8;
        }
    }

    number
}

impl TarFS {
    pub fn find_offset(devfd: &mut FileDescriptor, path: &str) -> Option<(usize, usize)> {
        // iterate over the structure:

        let mut buffer: [u8; HEADER_SIZE] = [0; HEADER_SIZE];
        let mut block_no = 0;

        unsafe {
            let devfs_driver = DevFSDriver::new();
            // open the device:
            loop {
                let read_result = devfs_driver.read(devfd, &mut buffer);
                if read_result.is_err() {
                    log::debug!(
                        "Disk IO error when reading from Tarfs, err={:?}",
                        read_result.unwrap_err()
                    );
                    return None;
                }

                let (head, body, _tail) = buffer.align_to::<TarHeader>();
                assert_eq!(head.is_empty(), true);
                let tar_header = &body[0];

                let signature = str::from_utf8_unchecked(&tar_header.signature);

                if !signature.starts_with("ustar") {
                    break;
                }

                let read_path = str::from_utf8_unchecked(&tar_header.name);
                // check path, are they equal?
                if read_path.starts_with(path) {
                    let file_entry = (block_no + 1) * mem::size_of::<TarHeader>();
                    return Some((file_entry, oct_to_usize(&tar_header.size)));
                }

                let to_skip_bytes = oct_to_usize(&tar_header.size);
                block_no += (to_skip_bytes + HEADER_SIZE - 1) / HEADER_SIZE;
                block_no += 1;

                let seek_result = devfs_driver.seek(devfd, (block_no * HEADER_SIZE) as u32);
                if seek_result.is_err() {
                    log::debug!("IO error on disk seek");
                    break;
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct TarFSDriver {
    pub device: String,
}

impl TarFSDriver {
    pub fn new_from_drive(device: &str) -> Self {
        TarFSDriver {
            device: String::from(device),
        }
    }
}

impl FSOps for TarFSDriver {
    fn open(&mut self, path: &str, flags: u32) -> Result<FileDescriptor, FSError> {
        let path = format!("/tarfs/{}", path);

        let mut devfs_driver = DevFSDriver::new();

        let devfd_result = devfs_driver.open(&self.device, 0);
        if devfd_result.is_err() {
            log::debug!("error=Attempt to open unknown device {}", self.device);
            return Err(FSError::NotFound);
        }

        let mut devfd = devfd_result.unwrap();
        if let Some((offset, size)) = TarFS::find_offset(&mut devfd, &path) {
            let _ = devfs_driver.close(&devfd);
            return Ok(FileDescriptor::TarFSNode(TarFileDescriptor {
                offset,
                size,
                flags,
                seeked_offset: 0,
            }));
        }

        let _ = devfs_driver.close(&devfd);
        Err(FSError::NotFound)
    }

    fn close(&self, _fd: &FileDescriptor) -> Result<(), FSError> {
        // a stub
        Ok(())
    }
}

impl FDOps for TarFSDriver {
    fn write(&self, _fd: &mut FileDescriptor, _buffer: &[u8]) -> Result<usize, FSError> {
        // tarfs doens't support writes, it is a readonly file-system.
        Err(FSError::InvalidOperation)
    }

    fn read(&self, _fd: &mut FileDescriptor, _buffer: &mut [u8]) -> Result<usize, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn seek(&self, _fd: &mut FileDescriptor, _offset: u32) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
}
