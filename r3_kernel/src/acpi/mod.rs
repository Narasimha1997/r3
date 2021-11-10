pub mod rsdt;

pub fn init() {
    rsdt::setup_acpi();
}