#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)]
#![feature(asm)] // enable asm
#![feature(alloc_error_handler)] // enable allocation errors
#![feature(naked_functions)] // allow naked calling convention
#![feature(llvm_asm)]

extern crate bootloader;
extern crate log;

pub mod acpi;
pub mod boot_proto;
pub mod cpu;
pub mod drivers;
pub mod logging;
pub mod mm;
pub mod system;

use boot_proto::BootProtocol;
use bootloader::BootInfo;

/// This function is called on panic.

fn init_basic_setup(boot_info: &'static BootInfo) {
    BootProtocol::create(boot_info);

    drivers::display::init();
    logging::init();

    log::info!("Hello, kernel world!");
    BootProtocol::print_boot_info();

    cpu::init_base_processor_tables();

    cpu::init_core_legacy_hardware();
    cpu::init_features_detection();
    cpu::run_test_breakpoint_recovery();

    mm::init();
    acpi::init();

    // init PCI device list.
    drivers::pci::detect_devices();

    // pit sleep for sometime:
    cpu::tsc::TSCSleeper::sleep_sec(1);

    log::info!("Initial stage booted properly.");
}

fn init_smp() {
    acpi::setup_smp_prerequisites();
    cpu::hw_interrupts::setup_post_apic_interrupts();

    system::timer::SystemTimer::start_ticks();
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init_basic_setup(boot_info);
    init_smp();

    cpu::halt_with_interrupts();
}
