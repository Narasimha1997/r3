pub mod ioapic;
pub mod lapic;
pub mod madt;
pub mod power;
pub mod rsdt;

pub fn init() {
    rsdt::setup_acpi();
    madt::setup_madt();
}

pub fn setup_smp_prerequisites() {
    // enable LAPIC for base processor.
    // enable ioapics for handling external device interrupts
    ioapic::init_io_apics();
    lapic::init_bsp_lapic();
    assert_eq!(lapic::bsp_apic_enabled(), true);
}
