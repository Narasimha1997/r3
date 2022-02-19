pub mod lapic;
pub mod madt;
pub mod power;
pub mod rsdt;
pub mod ioapic;

pub fn init() {
    rsdt::setup_acpi();
    madt::setup_madt();
}

pub fn setup_smp_prerequisites() {
    // enable LAPIC for base processor.
    lapic::init_bsp_lapic();

    assert_eq!(lapic::bsp_apic_enabled(), true);
}
