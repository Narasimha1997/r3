extern crate alloc;
extern crate log;

use crate::mm::Alignment;
use crate::system::filesystem::devfs::DevFSDriver;
use crate::system::filesystem::vfs::FILESYSTEM;
use crate::system::filesystem::MountInfo;
use crate::system::filesystem::{FDOps, FSOps};
use crate::system::filesystem::{FSError, FileDescriptor};

use alloc::{format, string::String, vec::Vec};
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
    /// driver name
    pub driver_name: String,
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
        let path = format!("tarfs{}", path);

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
                driver_name: self.device.clone(),
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

    fn read(&self, fd: &mut FileDescriptor, buffer: &mut [u8]) -> Result<usize, FSError> {
        let mut dev_driver = DevFSDriver::new();
        let dev_result = dev_driver.open(&self.device, 0);

        if dev_result.is_err() {
            return Err(FSError::DeviceNotFound);
        }
        let mut dev_handle = dev_result.unwrap();

        match fd {
            FileDescriptor::TarFSNode(tarfd) => {
                let dest_slice = if buffer.len() > tarfd.size - tarfd.seeked_offset {
                    &mut buffer[0..(tarfd.size - tarfd.seeked_offset)]
                } else {
                    buffer
                };

                // how many number of blocks should I read?
                let start_offset = tarfd.seeked_offset;
                let end_offset = start_offset + dest_slice.len();

                // align them to 512 blocks
                let aligned_start = Alignment::align_down(start_offset as u64, 512) as usize;
                let aligned_end = Alignment::align_up(end_offset as u64, 512) as usize;

                let first_seek_offset = tarfd.seeked_offset
                    - Alignment::align_down(tarfd.seeked_offset as u64, 512) as usize;
                let end_slice_offset = aligned_end - end_offset;

                // how many blocks:
                let n_blocks: usize = (aligned_end - aligned_start) / 512;

                // read these blocks starting from offset
                let mut block_data: Vec<u8> = Vec::new();
                block_data.resize(512, 0);

                if n_blocks == 1 {
                    // read this block
                    // seek to the location

                    let seek_result =
                        dev_driver.seek(&mut dev_handle, (tarfd.offset + aligned_start) as u32);
                    if seek_result.is_err() {
                        return Err(FSError::InvalidSeek);
                    }

                    let read_result = dev_driver.read(&mut dev_handle, &mut block_data);
                    if read_result.is_err() {
                        return Err(FSError::IOError);
                    }

                    // copy this data to the slice
                    dest_slice.clone_from_slice(
                        &block_data[tarfd.seeked_offset..(tarfd.size - tarfd.seeked_offset)],
                    );

                    return Ok(dest_slice.len());
                }

                let read_result = dev_driver.read(&mut dev_handle, &mut block_data);
                if read_result.is_err() {
                    return Err(FSError::IOError);
                }

                // first block
                dest_slice[0..first_seek_offset]
                    .clone_from_slice(&block_data[tarfd.seeked_offset..]);

                // copy other blocks completely
                if n_blocks > 2 {
                    for idx in 1..(n_blocks - 1) {
                        let seek_result = dev_driver.seek(
                            &mut dev_handle,
                            (tarfd.offset + aligned_start + (idx * 512)) as u32,
                        );

                        if seek_result.is_err() {
                            return Err(FSError::InvalidSeek);
                        }

                        let read_result = dev_driver.read(&mut dev_handle, &mut block_data);
                        if read_result.is_err() {
                            return Err(FSError::IOError);
                        }
                        dest_slice[512 + ((idx - 1) * 512)..512 + (idx * 512)]
                            .clone_from_slice(&block_data);
                    }
                }

                let n_read = (n_blocks - 1) * 512;
                let seek_result = dev_driver.seek(
                    &mut dev_handle,
                    (tarfd.offset + aligned_start + (n_read * 512)) as u32,
                );

                if seek_result.is_err() {
                    return Err(FSError::InvalidSeek);
                }

                // last block
                let read_result = dev_driver.read(&mut dev_handle, &mut block_data);
                if read_result.is_err() {
                    return Err(FSError::IOError);
                }

                dest_slice[n_read..n_read + end_slice_offset]
                    .clone_from_slice(&block_data[0..end_slice_offset]);

                return Ok(dest_slice.len());
            }
            _ => {}
        }
        Err(FSError::NotFound)
    }

    fn seek(&self, fd: &mut FileDescriptor, offset: u32) -> Result<(), FSError> {
        match fd {
            FileDescriptor::TarFSNode(tarfd) => {
                if tarfd.seeked_offset + offset as usize > tarfd.size {
                    return Err(FSError::InvalidSeek);
                }

                tarfd.seeked_offset = tarfd.seeked_offset + offset as usize;
                return Ok(());
            }
            _ => {}
        }

        return Err(FSError::NotFound);
    }
}

pub fn mount_tarfs(device: &str, path: &str) {
    let mut fs_lock = FILESYSTEM.lock();
    let tarfs = TarFSDriver::new_from_drive(device);
    let mount_info = MountInfo::TarFS(tarfs);

    fs_lock
        .mount_at(path, mount_info)
        .expect("Failed to mount tarfs");
    log::info!("Mounted tarfs at {}", path);
}
