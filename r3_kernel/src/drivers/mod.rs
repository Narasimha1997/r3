extern crate alloc;
extern crate log;

use crate::system::filesystem::devfs::register_device;
use alloc::boxed::Box;

pub mod disk;
pub mod display;
pub mod pci;
pub mod uart;

/// registers all the devices to DevFS
pub fn register_drivers() {
    // mount uart:
    register_device("uart", 1, 0, Box::new(uart::UartIODriver::empty()))
        .expect("Failed to register devices to devfs");
    log::info!("Registered devfs devices - uart");
}
