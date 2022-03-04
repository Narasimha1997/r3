pub mod ioapic;
pub mod lapic;
pub mod madt;
pub mod power;
pub mod rsdt;

use crate::cpu;

pub fn init() {
    log::info!("enabling APIC");
    rsdt::setup_acpi();
    madt::setup_madt();
}

pub fn setup_smp_prerequisites() {
    // enable LAPIC for base processor.
    // enable ioapics for handling external device interrupts

    // disable interrupts
    cpu::disable_interrupts();

    cpu::pic::disable_legacy_interrupts();
    lapic::init_bsp_lapic();
    assert_eq!(lapic::bsp_apic_enabled(), true);

    let masked_interrupts  = [0, 2];

    ioapic::init_io_apics(&masked_interrupts);
    cpu::enable_interrupts();
}
