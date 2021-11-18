pub mod ata_pio;

pub fn init() {
    ata_pio::ATAController::probe_pci();
}
