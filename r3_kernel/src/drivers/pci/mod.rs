extern crate alloc;
extern crate bit_field;
extern crate log;
extern crate spin;

use crate::cpu;
use alloc::vec::Vec;
use bit_field::BitField;
use cpu::io::Port;
use lazy_static::lazy_static;
use spin::Mutex;

/// Refers to the address of PCI data port in PCI config space.
const PCI_DATA_PORT: usize = 0xCFC;

/// Refers to the address of PCI address port in PCI config space.
const PCI_ADDRESS_PORT: usize = 0xCF8;

/// Refers to the base address which is ORed with device specific ID, bus and function
const PCI_BASE_ADDR: usize = 0x80000000;

/// number of bus lines in PCI
const MAX_BUS: usize = 256;

/// number of devices per each bus
const MAX_DEVICES_PER_BUS: usize = 32;

/// number of functions per device
const MAX_FUNCTIONS_PER_DEVICE: usize = 8;

/// if this flag is set, then the device is a multi-function device
const FLAG_MULTIFUNCTION_DEVICE: usize = 80;

/// This function will be called upon every successfull device/function detection
/// on the system.
type OnEntryCallback = fn(bus: u8, dev: u8, func: u8);

#[derive(Clone, Copy)]
pub struct PCIConfigRegister {
    pub address_line: Port,
    pub data_line: Port,
    pub dev_addr: u32,
}

impl PCIConfigRegister {
    #[inline]
    fn get_address(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
        // https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_.231
        PCI_BASE_ADDR as u32
            | ((bus as u32) << 16)
            | ((dev as u32) << 11 as u32)
            | ((func as u32) << 8 as u32)
            | ((offset as u32) & 0xFC)
    }

    pub fn new(bus: u8, dev: u8, func: u8, offset: u8) -> Self {
        PCIConfigRegister {
            address_line: Port::new(PCI_ADDRESS_PORT, false),
            data_line: Port::new(PCI_DATA_PORT, false),
            dev_addr: Self::get_address(bus, dev, func, offset),
        }
    }

    pub fn read_config(&self) -> u32 {
        self.address_line.write_u32(self.dev_addr);
        self.data_line.read_u32()
    }

    pub fn write_config(&self, data: u32) {
        self.address_line.write_u32(self.dev_addr);
        self.data_line.write_u32(data);
    }
}

/// Handles different types of queries on PCI devices.
pub enum PCIDeviceQuery {
    DeviceID,
    VendorID,
    HeaderType,
}

impl PCIDeviceQuery {
    pub fn query(&self, bus: u8, dev: u8, func: u8) -> u16 {
        match self {
            Self::DeviceID => PCIConfigRegister::new(bus, dev, func, 0x00)
                .read_config()
                .get_bits(0..16) as u16,
            Self::VendorID => PCIConfigRegister::new(bus, dev, func, 0x00)
                .read_config()
                .get_bits(16..32) as u16,
            Self::HeaderType => PCIConfigRegister::new(bus, dev, func, 0x0C)
                .read_config()
                .get_bits(16..24) as u16,
        }
    }
}

/// DeviceProber contains functions used to probe PCI devices on host machine.
pub struct PCIDeviceProber;

impl PCIDeviceProber {
    #[inline]
    /// the PCI host controller will return 16-bit 1s if the
    /// device is non-existing at that config location.
    fn is_empty(config_word: u16) -> bool {
        config_word == 0xFFFF
    }

    #[inline]
    fn is_multi_function(config_word: u16) -> bool {
        config_word & FLAG_MULTIFUNCTION_DEVICE as u16 != 0
    }

    #[inline]
    pub fn probe_device(bus: u8, dev: u8, callback: OnEntryCallback) {
        let vendor_id = PCIDeviceQuery::VendorID.query(bus, dev, 0);
        if Self::is_empty(vendor_id) {
            return;
        }

        callback(bus, dev, 0);

        // is this a multi-function device?
        let header_type = PCIDeviceQuery::HeaderType.query(bus, dev, 0);
        if Self::is_multi_function(header_type) {
            for func in 0..MAX_FUNCTIONS_PER_DEVICE {
                let vendor_id = PCIDeviceQuery::VendorID.query(bus, dev, func as u8);
                if !Self::is_empty(vendor_id) {
                    callback(bus, dev, func as u8);
                }
            }
        }
    }

    #[inline]
    pub fn probe_bus(bus: u8, callback: OnEntryCallback) {
        for dev in 0..MAX_DEVICES_PER_BUS {
            Self::probe_device(bus, dev as u8, callback);
        }
    }

    pub fn probe(callback: OnEntryCallback) {
        for bus in 0..MAX_BUS {
            Self::probe_bus(bus as u8, callback);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PCIDeviceInterruptInfo {
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct PCIDeviceControlRegs {
    pub command: u16,
    pub status: u16,
}

#[derive(Clone, Debug, Copy)]
pub struct PCIDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bars: [u32; 6],
}

impl PCIDevice {
    pub fn new(bus: u8, dev: u8, func: u8) -> PCIDevice {
        let vendor_id = PCIDeviceQuery::VendorID.query(bus, dev, func);
        let device_id = PCIDeviceQuery::DeviceID.query(bus, dev, func);

        let mut bars: [u32; 6] = [0; 6];

        for idx in 0..6 {
            let offset = 0x10 + ((idx as u8) << 2);
            let config_reg = PCIConfigRegister::new(bus, dev, func, offset);
            bars[idx] = config_reg.read_config();
        }

        PCIDevice {
            bus,
            dev,
            func,
            vendor_id,
            device_id,
            bars,
        }
    }

    pub fn control_registers(&self) -> PCIDeviceControlRegs {
        let config_reg = PCIConfigRegister::new(self.bus, self.dev, self.func, 0x04);
        let data = config_reg.read_config();
        PCIDeviceControlRegs {
            command: data.get_bits(0..16) as u16,
            status: data.get_bits(16..32) as u16,
        }
    }

    pub fn interrupt_info(&self) -> PCIDeviceInterruptInfo {
        let config_reg = PCIConfigRegister::new(self.bus, self.dev, self.func, 0x3C);
        let data = config_reg.read_config();
        PCIDeviceInterruptInfo {
            interrupt_line: data.get_bits(0..8) as u8,
            interrupt_pin: data.get_bits(8..16) as u8,
        }
    }

    pub fn write_config(&self, offset: u8, value: u32) {
        let config_reg = PCIConfigRegister::new(self.bus, self.dev, self.func, offset);
        config_reg.write_config(value);
    }
}

lazy_static! {
    pub static ref PCI_DEVICES: Mutex<Vec<PCIDevice>> = Mutex::new(Vec::new());
}

fn on_device_callback(bus: u8, dev: u8, func: u8) {
    let pci_device = PCIDevice::new(bus, dev, func);

    log::info!(
        "New PCI device added. bus={:x}, dev={:x}, func={:x}
         vendor_id={:x}, device_id={:x}",
        pci_device.bus,
        pci_device.dev,
        pci_device.func,
        pci_device.vendor_id,
        pci_device.device_id
    );

    PCI_DEVICES.lock().push(pci_device);
}

pub fn detect_devices() {
    PCIDeviceProber::probe(on_device_callback);
}

pub fn search_device(vendor_id: u16, device_id: u16) -> Option<PCIDevice> {
    for &pci_dev in PCI_DEVICES.lock().iter() {
        if pci_dev.vendor_id == vendor_id && pci_dev.device_id == device_id {
            return Some(pci_dev);
        }
    }

    None
}