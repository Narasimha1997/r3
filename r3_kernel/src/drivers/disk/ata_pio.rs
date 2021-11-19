extern crate alloc;
extern crate bit_field;
extern crate log;
extern crate spin;

use crate::cpu::io::Port;
use crate::drivers::pci::{search_device, PCIDevice};
use crate::system::timer::{wait_ns, Time};
use bit_field::BitField;

use core::iter::Iterator;

use alloc::{string::String, vec::Vec};
use spin::Mutex;

use lazy_static::lazy_static;

/// ATA Device ID in PCI device bus
const ATA_DEVICE_ID: usize = 0x7010;

/// ATA vendor ID
const ATA_VENDOR_ID: usize = 0x8086;

/// ATA soft-reset bit
const ATA_SOFT_RESET: u8 = 0x04;

/// ATA block size - made public because others (ex: fs) may use it.
pub const ATA_BLOCK_SIZE: usize = 512;

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ATADriveType {
    PRIMARY = 0xA0,
    SECONDARY = 0xB0,
}

#[derive(Debug, Clone)]
pub struct ATADrive {
    pub bus_no: u8,
    pub drive_type: ATADriveType,
    pub n_blocks: u32,
    pub model_name: String,
    pub serial_no: String,
}

impl ATADrive {
    #[inline]
    pub fn size(&self) -> usize {
        self.n_blocks as usize * ATA_BLOCK_SIZE as usize
    }

    #[inline]
    pub fn dump(&self) {
        log::info!(
            "ATA:{}_{:?} model={}, serial={}",
            self.bus_no,
            match self.drive_type {
                ATADriveType::PRIMARY => "primary",
                ATADriveType::SECONDARY => "secondary",
            },
            self.model_name,
            self.serial_no
        )
    }
}

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
pub enum ATACommand {
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
        self.regs.drive.write_u8(ATADriveType::PRIMARY as u8);
    }

    #[inline]
    pub fn sel_secondary(&self) {
        self.regs.drive.write_u8(ATADriveType::SECONDARY as u8);
    }

    #[inline]
    pub fn approx_400ns_wait(&self) {
        // reads from alt status register,
        // each read if assumed to be 100ns.
        self.regs.alt_status.read_u8();
        self.regs.alt_status.read_u8();
        self.regs.alt_status.read_u8();
        self.regs.alt_status.read_u8();
    }

    #[inline]
    pub fn send_command(&self, cmd: ATACommand) {
        self.regs.cmd.write_u8(cmd as u8);
    }

    #[inline]
    pub fn soft_reset(&self) {
        self.regs.ctrl.write_u8(ATA_SOFT_RESET);
        wait_ns(5 * Time::MilliSecond as u64);
        self.regs.ctrl.write_u8(0);
        wait_ns(5 * Time::MilliSecond as u64);
    }

    #[inline]
    pub fn status(&self) -> u8 {
        self.regs.status.read_u8()
    }

    #[inline]
    pub fn is(&self, status: ATAStatus) -> bool {
        self.status().get_bit(status as usize)
    }

    #[inline]
    pub fn wait_while_busy(&self) {
        self.approx_400ns_wait();
        while self.is(ATAStatus::BSY) {
            self.approx_400ns_wait();
        }
    }

    #[inline]
    pub fn read_data_reg(&self) -> u16 {
        self.regs.data.read_u16()
    }

    #[inline]
    pub fn write_data_reg(&self, data: u16) {
        self.regs.data.write_u16(data);
    }

    #[inline]
    fn clear_lbas(&self) {
        self.regs.sector_count.write_u8(0);
        self.regs.lba0.write_u8(0);
        self.regs.lba1.write_u8(0);
        self.regs.lba2.write_u8(0);
    }

    #[inline]
    pub fn read_current_block(&self, buffer: &mut [u16]) {
        // since we have 16-bit wide data register
        for offset in 0..(ATA_BLOCK_SIZE / 2) {
            let current_data = self.read_data_reg();
            buffer[offset] = current_data;
        }
    }

    #[inline]
    pub fn write_current_block(&self, buffer: &[u16]) {
        for offset in 0..(ATA_BLOCK_SIZE / 2) {
            let current_data = buffer[offset];
            self.write_data_reg(current_data);
        }
    }

    #[inline]
    pub fn set_block(&self, drive: ATADriveType, block: u32) {
        let drive_id = drive as u8 + 64;
        let drv_bits = block.get_bits(24..28) as u8;

        self.regs.drive.write_u8(drive_id | drv_bits & 0x0F);
        self.regs.sector_count.write_u8(1);
        let lba0_bits = block.get_bits(0..8) as u8;
        let lba1_bits = block.get_bits(8..16) as u8;
        let lba2_bits = block.get_bits(16..24) as u8;

        self.regs.lba0.write_u8(lba0_bits);
        self.regs.lba1.write_u8(lba1_bits);
        self.regs.lba2.write_u8(lba2_bits);
    }

    #[inline]
    fn get_ata_info(&self, sector_0: &[u16; 256], drive_type: ATADriveType) -> Option<ATADrive> {
        let serial_no = sector_0[10..20]
            .iter()
            .map(|word| word.to_be_bytes().map(|byte| byte as char))
            .flatten()
            .collect::<String>()
            .trim()
            .into();
        let model_name: String = sector_0[27..47]
            .iter()
            .map(|word| word.to_be_bytes().map(|byte| byte as char))
            .flatten()
            .collect::<String>()
            .trim()
            .into();
        let n_blocks = (sector_0[61] as u32) << 16 | (sector_0[60] as u32);

        Some(ATADrive {
            bus_no: self.id,
            drive_type,
            model_name,
            serial_no,
            n_blocks,
        })
    }

    pub fn identify(&self, d_type: ATADriveType) -> Option<ATADrive> {
        self.clear_lbas();
        self.send_command(ATACommand::IDENTIFY);

        if self.status() == 0 {
            return None;
        }

        self.wait_while_busy();

        if self.regs.lba1.read_u8() != 0 || self.regs.lba2.read_u8() != 0 {
            return None;
        }

        for _ in 0..256 {
            // if error, return:
            if self.is(ATAStatus::ERR) {
                return None;
            }

            if self.is(ATAStatus::RDY) {
                // drive is ready, read data:
                let mut sector_0_data: [u16; 256] = [0; 256];
                self.read_current_block(&mut sector_0_data);

                // gather metadata
                return self.get_ata_info(&sector_0_data, d_type);
            }
        }

        return None;
    }

    pub fn identify_primary(&self) -> Option<ATADrive> {
        self.soft_reset();
        self.approx_400ns_wait();

        self.sel_primary();

        self.identify(ATADriveType::PRIMARY)
    }

    pub fn identify_secondary(&self) -> Option<ATADrive> {
        self.soft_reset();
        self.approx_400ns_wait();

        self.identify(ATADriveType::SECONDARY)
    }
}

lazy_static! {
    pub static ref ATA_DEVICES: Mutex<Vec<ATADevice>> = Mutex::new(Vec::new());
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
        let mut drives_lock = ATA_DEVICES.lock();
        drives_lock.push(drive);
    }
}

pub fn register_devices() {
    // register drive 0:
    let drive_0 = ATAController::new_drive(0x1F0, 0x3F6, 14, 0);
    let drive_1 = ATAController::new_drive(0x170, 0x376, 15, 1);

    ATAController::add_drive(drive_0);
    ATAController::add_drive(drive_1);
}

lazy_static! {
    pub static ref ATA_DRIVES: Mutex<Vec<Option<ATADrive>>> = Mutex::new(Vec::new());
}

pub fn probe_drives() {
    let devices_lock = ATA_DEVICES.lock();
    let mut drives_lock = ATA_DRIVES.lock();

    for device in devices_lock.iter() {
        let primary_drive = device.identify_primary();
        let secondary_drive = device.identify_secondary();

        drives_lock.push(primary_drive);
        drives_lock.push(secondary_drive);
    }

    log::info!("Probed ATA PCI drives.");
}

pub fn list_drives() {
    let drives_lock = ATA_DRIVES.lock();
    for drive_opt in drives_lock.iter() {
        if let Some(drive) = drive_opt {
            drive.dump();
        }
    }
}
