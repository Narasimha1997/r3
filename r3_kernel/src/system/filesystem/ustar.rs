extern crate alloc;
extern crate log;

use crate::system::filesystem::devfs::DevFSDriver;
use crate::system::filesystem::FDOps;
use crate::system::filesystem::FileDescriptor;

use alloc::string::String;
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

#[inline]
fn oct_to_usize(buffer: &[u8]) -> usize {
    let mut multiplier = 1;
    let mut number = 0;
    let last_index = buffer.len();

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
    pub fn find_offset(devfd: &mut FileDescriptor, path: &str) -> Option<u64> {
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

                if str::from_utf8_unchecked(&tar_header.signature) != "ustar" {
                    break;
                }

                // check path, are they equal?
                if str::from_utf8_unchecked(&tar_header.name) == path {
                    let file_entry = (block_no + 1) * mem::size_of::<TarHeader>();
                    return Some(file_entry as u64);
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
pub struct TarFileDescriptor {
    /// name of the device on which the filesystem is formatted.
    pub dev_name: String,
    /// offset of the file on that device
    pub offset: u64,
}
