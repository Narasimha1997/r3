extern crate log;

use crate::drivers::pci::{search_device, PCIDevice};

/// ATA Device ID in PCI device bus
const ATA_DEVICE_ID: usize = 0x7010;

/// ATA vendor ID
const ATA_VENDOR_ID: usize = 0x8086;


#[repr(u8)]
pub enum ATAStatus {
    ERR = 0,
    IDX = 1,
    CORR = 2,
    DRQ = 3,
    SRV = 4,
    DF = 5,
    RDY = 6,
    BSY = 7,
}

#[repr(u8)]
pub enum ATACommands {
    IDENTIFY = 0xEC,
    READ = 0x20,
    WRITE = 0x30,
}


#[derive(Debug, Clone)]
/// Represents a physical ATA drive installed on this PC
pub struct ATADrive {

}

pub struct ATAController;



impl ATAController {
    #[inline]
    pub fn probe_pci() -> Option<PCIDevice> {
        let probe_result = search_device(ATA_VENDOR_ID as u16, ATA_DEVICE_ID as u16);
        if probe_result.is_none() {
            log::warn!("ATA device not found");
            return None;
        }

        probe_result
    }

    #[inline]
    pub fn new() {

    }
}
