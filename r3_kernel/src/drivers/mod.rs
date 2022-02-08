extern crate alloc;
extern crate log;

use crate::system::filesystem::devfs::register_device;
use alloc::{boxed::Box, vec::Vec};

use crate::system::net::iface::PhyNetDevType;

pub mod disk;
pub mod display;
pub mod keyboard;
pub mod pci;
pub mod random;
pub mod rtl8139;
pub mod tty;
pub mod uart;

/// registers all the devices to DevFS
pub fn register_buultin_devices() {
    // mount uart:
    register_device("serial", 1, 0, Box::new(uart::UartIODriver::empty()))
        .expect("Failed to register UART to devfs");

    register_device("tty", 1, 1, Box::new(tty::TTYDriver::empty()))
        .expect("Failed to register TTY to devfs");

    register_device("rand", 1, 2, Box::new(random::RandomIODriver::empty()))
        .expect("Failed to register Random generator to devfs");

    log::info!("Registered devfs devices - uart");
}

/// The ATA controller device
const ATA_CONTROLLER: (u16, u16) = (0x7010, 0x8086);
const RTL_NETWORK_INTERFACE: (u16, u16) = (0x8139, 0x10EC);

/// this method iterates over the available PCI devices,
/// uses vendor_id and device_id to determine which driver can
/// serve this device.
pub fn load_pci_drivers() {
    let mut devices: Vec<(u16, u16)> = Vec::new();
    for &device in pci::PCI_DEVICES.lock().iter() {
        devices.push((device.device_id, device.vendor_id));
    }

    for (device_id, vendor_id) in devices.iter() {
        match (*device_id, *vendor_id) {
            ATA_CONTROLLER => {
                // load the ATA controller driver
                log::info!("Found driver for device {:x}:{:x}.", device_id, vendor_id);
                disk::init();
                disk::register_hdd_devices();
            }
            RTL_NETWORK_INTERFACE => {
                log::info!("Found driver for device {:x}:{:x}", device_id, vendor_id);
                // rtl8139::init();
            }
            _ => {
                log::warn!(
                    "No driver found to handle the device {:x}:{:x}",
                    device_id,
                    vendor_id
                );
            }
        }
    }
}

pub fn get_network_device() -> Option<Box<PhyNetDevType>> {
    // TODO: Extend this to more network
    // check if there is a RTL driver:
    let (device_id, vendor_id) = RTL_NETWORK_INTERFACE;
    let rtl_driver_opt = pci::search_device(vendor_id, device_id);
    if rtl_driver_opt.is_none() {
        return None;
    }

    // we found the device: create the instance
    let mut device = rtl8139::Realtek8139Device::new();
    device.prepare_interface();

    Some(Box::new(device))
}
