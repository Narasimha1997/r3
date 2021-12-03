#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)]
#![feature(asm)] // enable asm
#![feature(alloc_error_handler)] // enable allocation errors
#![feature(naked_functions)] // allow naked calling convention
#![feature(llvm_asm)]

extern crate alloc;
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

use alloc::format;

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

    // init PCI device list.
    drivers::pci::detect_devices();

    acpi::init();

    log::info!("Initial stage booted properly.");
}

fn ideal_k_thread() {
    cpu::halt_with_interrupts();
}

fn start_idle_kthread() {
    // this will always run in the background and keep atleast
    // one task running in the kernel with CPU interrupts enabled.
    let process = system::process::new(format!("kernel_background"), false);

    // start a thread for this process
    let thread_result = system::thread::new_from_function(
        &process,
        format!("idle_thread"),
        mm::VirtualAddress::from_u64(ideal_k_thread as fn() as u64),
    );

    if thread_result.is_err() {
        log::error!("Failed to run system idle thread, threading not working!!!");
        return;
    }

    // run this thread
    system::thread::run_thread(&thread_result.unwrap());
    log::info!("Started system idle thread in background.")
}

fn init_filesystem() {
    system::init_fs();
    drivers::register_drivers();

    system::init_tarfs();
}

fn init_functionalities() {
    acpi::setup_smp_prerequisites();
    cpu::hw_interrupts::setup_post_apic_interrupts();
    cpu::syscall::setup_syscall_interrupt();

    // init ATA device
    drivers::disk::init();
    init_filesystem();
    system::init_tasking();

    start_idle_kthread();
    system::timer::SystemTimer::start_ticks();
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init_basic_setup(boot_info);
    init_functionalities();

    cpu::halt_with_interrupts();
}
