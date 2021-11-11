pub mod rsdt;
pub mod madt;

pub fn init() {
    rsdt::setup_acpi();
    madt::setup_madt();
}