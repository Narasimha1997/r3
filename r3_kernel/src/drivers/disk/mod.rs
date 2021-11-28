extern crate log;

use crate::system::filesystem::devfs::{register_device, DevOps};
use crate::system::filesystem::FSError;

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

pub struct ATAIODriver;

impl ATAIODriver {
    pub fn empty() -> Self {
        ATAIODriver {}
    }
}

impl DevOps for ATAIODriver {
    fn write(&self, buffer: &[u8]) -> Result<usize, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn ioctl(&self, command: u8) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
}
