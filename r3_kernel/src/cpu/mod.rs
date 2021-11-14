pub mod cpuid;
pub mod exceptions;
pub mod hw_interrupts;
pub mod interrupts;
pub mod io;
pub mod mmu;
pub mod pic;
pub mod pit;
pub mod rflags;
pub mod segments;
pub mod tsc;
pub mod state;

pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

pub fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

pub fn are_enabled() -> bool {
    rflags::RFlags::is_set(rflags::RFlagsStruct::INTERRUPT_FLAG)
}

pub fn create_breakpoint() {
    unsafe {
        asm!("int3", options(nomem, nostack));
    }
}

pub fn halt() {
    unsafe {
        asm!("hlt");
    }
}

pub fn halt_with_interrupts() -> ! {
    enable_interrupts();
    unsafe {
        loop {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

pub fn halt_no_interrupts() -> ! {
    disable_interrupts();
    unsafe {
        loop {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

pub fn init_base_processor_tables() {
    segments::init_gdt();
    exceptions::init_exceptions();
}

pub fn init_features_detection() {
    // this will call the lazy static to initialize
    cpuid::display_features();
    cpuid::assert_min_levels();
}

pub fn run_test_breakpoint_recovery() {
    create_breakpoint();
    log::info!("Recovered from breakpoint, interrupts properly working.");
}

pub fn init_core_legacy_hardware() {
    pic::setup_pics();
    hw_interrupts::setup_hw_interrupts();

    // enable legacy interrupts:
    pic::enable_legacy_interrupts();
    log::info!("Enabled legacy PIC chip.");
    tsc::init_timer();
}
