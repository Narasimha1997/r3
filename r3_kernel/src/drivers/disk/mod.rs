extern crate log;
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
