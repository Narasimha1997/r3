extern crate log;

use crate::cpu::io::Port;
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


pub struct ATADevice;

pub struct ATAController;

impl ATAController {
    #[inline]
    pub fn probe_pci() -> Option<PCIDevice> {
        let probe_result = search_device(ATA_VENDOR_ID as u16, ATA_DEVICE_ID as u16);
        if probe_result.is_none() {
            log::warn!("ATA device not found");
            return None;
        }

        log::info!("ATA device: {:?}", probe_result);
        probe_result
    }
}
