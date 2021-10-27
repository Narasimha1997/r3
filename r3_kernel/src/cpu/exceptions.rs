extern crate log;

use crate::cpu;
use cpu::interrupts::{prepare_default_handle, prepare_no_ret_error_code_handle};
use cpu::interrupts::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;

// implements basic exception handlers:

pub extern "x86-interrupt" fn divide_by_zero(stk: InterruptStackFrame) {
    log::error!("Divide by zero exception\nException info: {:#?}", stk);
}

pub extern "x86-interrupt" fn breakpoint(stk: InterruptStackFrame) {
    log::error!("Breakpoint exception\nException info: {:#?}", stk);
}

pub extern "x86-interrupt" fn invalid_opcode(stk: InterruptStackFrame) {
    log::error!("Invalid opcode exception\nException info: {:#?}", stk);
}

pub extern "x86-interrupt" fn double_fault(stk: InterruptStackFrame, err: u64) -> ! {
    log::error!("Double fault exception {}\nException info: {:#?}", err, stk);
    cpu::halt_no_interrupts();
}

pub fn prepare_idt() -> InterruptDescriptorTable {
    let mut idt = InterruptDescriptorTable::empty();
    idt.divide_error = prepare_default_handle(divide_by_zero);
    idt.invalid_opcode = prepare_default_handle(invalid_opcode);
    idt.breakpoint = prepare_default_handle(breakpoint);
    idt.double_fault = prepare_no_ret_error_code_handle(double_fault);

    log::info!("Prepared basic exceptions.");
    return idt;
}

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = prepare_idt();
}

pub fn init_exceptions() {
    
}
