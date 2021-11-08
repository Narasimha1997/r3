extern crate log;
extern crate spin;

use crate::cpu;
use cpu::interrupts::{
    prepare_default_handle, prepare_no_ret_error_code_handle, prepare_page_fault_handler,
};
use cpu::interrupts::{InterruptDescriptorTable, InterruptStackFrame};
use cpu::mmu::{read_cr2, PageFaultExceptionTypes};
use lazy_static::lazy_static;
use spin::Mutex;

// implements basic exception handlers:

pub extern "x86-interrupt" fn divide_by_zero(stk: InterruptStackFrame) {
    log::error!("Divide by zero exception\nException info: {:#?}", stk);
}

extern "x86-interrupt" fn breakpoint(stk: InterruptStackFrame) {
    log::error!("Breakpoint exception\nException info: {:#?}", stk);
}

extern "x86-interrupt" fn invalid_opcode(stk: InterruptStackFrame) {
    log::error!("Invalid opcode exception\nException info: {:#?}", stk);
}

extern "x86-interrupt" fn double_fault(stk: InterruptStackFrame, err: u64) -> ! {
    log::error!("Double fault exception {}\nException info: {:#?}", err, stk);
    cpu::halt_no_interrupts();
}

extern "x86-interrupt" fn page_fault(stk: InterruptStackFrame, err: PageFaultExceptionTypes) -> ! {
    let cr2_val = read_cr2();
    // log exception
    log::error!(
        "Page Fault Exception:\n
        error_code={:?}, accessed_address=0x{:x},
        stack_frame={:?}",
        err,
        cr2_val,
        stk
    );

    cpu::halt_no_interrupts();
}

pub fn prepare_idt() -> InterruptDescriptorTable {
    let mut idt = InterruptDescriptorTable::empty();
    idt.divide_error = prepare_default_handle(divide_by_zero);
    idt.invalid_opcode = prepare_default_handle(invalid_opcode);
    idt.breakpoint = prepare_default_handle(breakpoint);
    idt.double_fault = prepare_no_ret_error_code_handle(double_fault);
    idt.page_fault = prepare_page_fault_handler(page_fault);

    idt.double_fault.set_stack_index(0);

    log::info!("Prepared basic exceptions.");
    return idt;
}

lazy_static! {
    pub static ref IDT: Mutex<InterruptDescriptorTable> = Mutex::new(prepare_idt());
}

pub fn init_exceptions() {
    // load processor IDT
    IDT.lock().load_into_cpu();
    log::info!("Initialized Interrupt descriptor table.");
}
