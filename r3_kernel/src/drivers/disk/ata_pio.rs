extern crate alloc;
extern crate log;
extern crate spin;

use crate::cpu::io::Port;
use crate::drivers::pci::{search_device, PCIDevice};

use alloc::vec::Vec;
use spin::Mutex;

use lazy_static::lazy_static;

/// ATA Device ID in PCI device bus
const ATA_DEVICE_ID: usize = 0x7010;

/// ATA vendor ID
const ATA_VENDOR_ID: usize = 0x8086;

/// ATA drive primary 
const ATA_DRIVE_PRIMARY: u8 = 0xA0;

/// ATA drive secondary
const ATA_DRIVE_SECONDARY: u8 = 0xB0;

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
pub struct ATARegisters {
    pub data: Port,
    pub error: Port,
    pub features: Port,
    pub sector_count: Port,
    pub lba0: Port,
    pub lba1: Port,
    pub lba2: Port,
    pub drive: Port,
    pub status: Port,
    pub alt_status: Port,
    pub cmd: Port,
    pub ctrl: Port,
    pub blockless: Port,
}

#[derive(Debug, Clone)]
/// Represents a physical ATA drive installed on this PC
pub struct ATADevice {
    pub id: u8,
    pub irq_no: u8,
    pub regs: ATARegisters,
}

impl ATADevice {
    #[inline]
    pub fn sel_primary(&self) {
        self.regs.drive.write_u8(ATA_DRIVE_PRIMARY);
    }

    #[inline]
    pub fn sel_secondary(&self) {
        self.regs.drive.write_u8(ATA_DRIVE_SECONDARY);
    }
}

lazy_static! {
    pub static ref ATA_DRIVES: Mutex<Vec<ATADevice>> = Mutex::new(Vec::new());
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
    pub fn new_drive(io_start: usize, ctrl_start: usize, irq_no: u8, id: u8) -> ATADevice {
        ATADevice {
            id,
            irq_no,
            regs: ATARegisters {
                data: Port::new(io_start + 0, false),
                error: Port::new(io_start + 1, false),
                features: Port::new(io_start + 1, false),
                sector_count: Port::new(io_start + 2, false),
                lba0: Port::new(io_start + 3, false),
                lba1: Port::new(io_start + 4, false),
                lba2: Port::new(io_start + 5, false),
                drive: Port::new(io_start + 6, false),
                status: Port::new(io_start + 7, false),
                alt_status: Port::new(ctrl_start + 0, false),
                cmd: Port::new(io_start + 7, false),
                ctrl: Port::new(ctrl_start + 0, false),
                blockless: Port::new(ctrl_start + 1, false),
            },
        }
    }

    #[inline]
    pub fn add_drive(drive: ATADevice) {
        let mut drives_lock = ATA_DRIVES.lock();
        drives_lock.push(drive);
    }
}

pub fn register_drives() {
    // register drive 0:
    let drive_0 = ATAController::new_drive(0x1F0, 0x3F6, 14, 0);
    let drive_1 = ATAController::new_drive(0x170, 0x376, 15, 1);

    ATAController::add_drive(drive_0);
    ATAController::add_drive(drive_1);
}
