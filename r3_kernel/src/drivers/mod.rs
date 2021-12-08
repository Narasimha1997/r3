extern crate alloc;
extern crate log;

use crate::system::filesystem::devfs::register_device;
use alloc::boxed::Box;

pub mod disk;
pub mod display;
pub mod pci;
pub mod uart;

/// registers all the devices to DevFS
pub fn register_buultin_devices() {
    // mount uart:
    register_device("serial", 1, 0, Box::new(uart::UartIODriver::empty()))
        .expect("Failed to register devices to devfs");
    log::info!("Registered devfs devices - uart");
}

/// The ATA controller device
const ATA_CONTROLLER: (u16, u16) = (0x7010, 0x8086);

/// this method iterates over the available PCI devices,
/// uses vendor_id and device_id to determine which driver can
/// serve this device.
pub fn load_pci_drivers() {
    for &device in pci::PCI_DEVICES.lock().iter() {
        let (device_id, vendor_id) = (device.device_id, device.vendor_id);
        match (device_id, vendor_id) {
            ATA_CONTROLLER => {
                // load the ATA controller driver
                log::info!("Found driver for device {}:{}.", device_id, vendor_id);
                disk::init();
                disk::register_hdd_devices();
            }
            _ => {
                log::warn!(
                    "No driver found to handle the device {}:{}",
                    device_id,
                    vendor_id
                );
            }
        }
    }
}
