pub mod exceptions;
pub mod interrupts;
pub mod io;
pub mod rflags;
pub mod segments;
pub mod cpuid;

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
