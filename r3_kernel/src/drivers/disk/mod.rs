extern crate alloc;
extern crate log;

use crate::system::filesystem::devfs::{register_device, DevFSDescriptor, DevOps};
use crate::system::filesystem::{FSError, SeekType};

use alloc::{boxed::Box, format};

pub mod ata_pio;

pub fn init() {
    if let Some(_) = ata_pio::ATAController::probe_pci() {
        // register devices
        ata_pio::register_devices();
        ata_pio::probe_drives();
        ata_pio::list_drives();
    } else {
        log::warn!("ATA controller not found on this machine.");
    }
}

pub struct ATAIODriver {
    pub index: usize,
}

impl ATAIODriver {
    pub fn empty(index: usize) -> Self {
        ATAIODriver { index }
    }
}

impl DevOps for ATAIODriver {
    fn write(&self, fd: &mut DevFSDescriptor, buffer: &[u8]) -> Result<usize, FSError> {
        let device_lock = ata_pio::ATA_DRIVES.lock();
        let device = device_lock.get(self.index).unwrap().as_ref().unwrap();
        let block_start = fd.offset / ata_pio::ATA_BLOCK_SIZE as u32;

        let is_short_block: bool;

        let n_blocks = if buffer.len() > ata_pio::ATA_BLOCK_SIZE {
            is_short_block = false;
            buffer.len() / ata_pio::ATA_BLOCK_SIZE
        } else {
            is_short_block = true;
            1
        };

        for block_offset in 0..n_blocks {
            let start = block_offset * ata_pio::ATA_BLOCK_SIZE;
            let end = if is_short_block {
                buffer.len()
            } else {
                start + ata_pio::ATA_BLOCK_SIZE
            };
            device.write_block(&buffer[start..end], block_start + block_offset as u32);
        }

        Ok(buffer.len())
    }

    fn read(&self, fd: &mut DevFSDescriptor, buffer: &mut [u8]) -> Result<usize, FSError> {
        let device_lock = ata_pio::ATA_DRIVES.lock();
        let device = device_lock.get(self.index).unwrap().as_ref().unwrap();
        let block_start = fd.offset / ata_pio::ATA_BLOCK_SIZE as u32;

        let is_short_block: bool;

        let n_blocks = if buffer.len() > ata_pio::ATA_BLOCK_SIZE {
            is_short_block = false;
            buffer.len() / ata_pio::ATA_BLOCK_SIZE
        } else {
            is_short_block = true;
            1
        };

        for block_offset in 0..n_blocks {
            let start = block_offset * ata_pio::ATA_BLOCK_SIZE;
            let end = if is_short_block {
                buffer.len()
            } else {
                start + ata_pio::ATA_BLOCK_SIZE
            };
            device.read_block(&mut buffer[start..end], block_start + block_offset as u32);
        }

        Ok(buffer.len())
    }

    fn ioctl(&self, _command: u8) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn seek(&self, fd: &mut DevFSDescriptor, offset: u32, st: SeekType) -> Result<u32, FSError> {
        match st {
            SeekType::SEEK_SET => {
                fd.offset = offset;
            }
            SeekType::SEEK_CUR => {
                fd.offset = fd.offset + offset;
            }
            SeekType::SEEK_END => {
                return Err(FSError::InvalidSeek);
            }
        }

        Ok(fd.offset as u32)
    }
}

pub fn register_hdd_devices() {
    let locked_drives = ata_pio::ATA_DRIVES.lock();
    for (index, drive_opt) in locked_drives.iter().enumerate() {
        if drive_opt.is_some() {
            // register this drive:
            let char_suffix = (97 + index) as u8 as char;
            let drive_name = format!("hd{}", char_suffix);
            // mount the driver:
            let driver = ATAIODriver::empty(index);

            register_device(&drive_name, 2, index as u32, Box::new(driver))
                .expect("Failed to register hard disk device to devfs");

            log::info!("Registered devfs device {}", drive_name);
        }
    }
}

unsafe impl Send for ATAIODriver {}
unsafe impl Sync for ATAIODriver {}
