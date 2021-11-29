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

fn thread_2() {
    /*let timeval: [u8; 16] = [0; 16];
    let mut result: u64 = 0;
    loop {
        unsafe {
            asm!(
                "int 0x80", in("rax")228,
                in("rdi")0, in("rsi")&timeval,
                in("rdx")0,
                lateout("rax") result
            )
        }

        result = result + 1;
    }*/
    let no = 255;
    loop {
        unsafe {
            asm!("int 0x80");
        }
    }
}

fn test_sample_tasking() {
    let pid1 = system::process::new("system_main".to_string(), false);

    let tid1 = system::thread::new_from_function(
        &pid1,
        "th_1".to_string(),
        mm::VirtualAddress::from_u64(ideal_k_thread as fn() as u64),
    );

    let pid2 = system::process::new("user_test".to_string(), true);

    let tid2 = system::thread::new_from_function(
        &pid2,
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
    cpu::syscall::setup_syscall_interrupt();

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
