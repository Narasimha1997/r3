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

use alloc::string::ToString;
use boot_proto::BootProtocol;
use bootloader::BootInfo;
use system::filesystem::{FDOps, FSOps};

use core::str;

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

fn thread_1() {
    let mut counter = 0;

    let mut hdb = system::filesystem::vfs::FILESYSTEM
        .lock()
        .open("/dev/hdb", 0)
        .unwrap();
    // seek to some block
    let _ = system::filesystem::vfs::FILESYSTEM
        .lock()
        .seek(&mut hdb, 512 * 100);

    let buffer_write = "Helloworld";
    let mut buffer_read: [u8; 11] = [0; 11];

    loop {
        if counter % 200 == 0 {
            log::info!("Thread-1: {}", counter);
        }

        for _ in 0..10000 {
            cpu::io::wait(1);
        }

        // read and write some block
        let _ = system::filesystem::vfs::FILESYSTEM
            .lock()
            .write(&mut hdb, &buffer_write.as_bytes());
        let _ = system::filesystem::vfs::FILESYSTEM
            .lock()
            .read(&mut hdb, &mut buffer_read);

        // log the string:
        log::info!("Read: {}", str::from_utf8(&buffer_read).unwrap());

        counter += 1;

        if counter % 10001 == 0 {
            system::tasking::exit(0);
        }
    }
}

fn thread_2() {
    let mut counter = 0;

    let mut hdb = system::filesystem::vfs::FILESYSTEM
        .lock()
        .open("/dev/hdb", 0)
        .unwrap();
    // seek to some block
    let _ = system::filesystem::vfs::FILESYSTEM
        .lock()
        .seek(&mut hdb, 512 * 100);

    let buffer_write = "Helloworld";
    let mut buffer_read: [u8; 11] = [0; 11];

    loop {
        if counter % 200 == 0 {
            log::info!("Thread-2: {}", counter);
        }

        for _ in 0..10000 {
            cpu::io::wait(1);
        }

        // read and write some block
        let _ = system::filesystem::vfs::FILESYSTEM
            .lock()
            .write(&mut hdb, &buffer_write.as_bytes());
        let _ = system::filesystem::vfs::FILESYSTEM
            .lock()
            .read(&mut hdb, &mut buffer_read);

        // log the string:
        log::info!("Read: {}", str::from_utf8(&buffer_read).unwrap());

        counter += 1;

        if counter % 10001 == 0 {
            system::tasking::exit(0);
        }
    }
}

fn test_sample_tasking() {
    let pid1 = system::process::new("system_test".to_string(), false);

    let tid1 = system::thread::new_from_function(
        &pid1,
        "th_1".to_string(),
        mm::VirtualAddress::from_u64(thread_1 as fn() as u64),
    );
    let tid2 = system::thread::new_from_function(
        &pid1,
        "th_2".to_string(),
        mm::VirtualAddress::from_u64(thread_2 as fn() as u64),
    );

    system::thread::run_thread(&tid1.unwrap());
    system::thread::run_thread(&tid2.unwrap());
}

fn init_filesystem() {
    system::init_fs();
    drivers::register_drivers();
}

fn init_functionalities() {
    acpi::setup_smp_prerequisites();
    cpu::hw_interrupts::setup_post_apic_interrupts();

    // init ATA device
    drivers::disk::init();
    init_filesystem();

    system::init_tasking();

    test_sample_tasking();

    system::timer::SystemTimer::start_ticks();
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init_basic_setup(boot_info);
    init_functionalities();

    cpu::halt_with_interrupts();
}
