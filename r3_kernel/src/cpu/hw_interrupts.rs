extern crate log;

use crate::cpu::exceptions;
use crate::cpu::interrupts;
use crate::cpu::pic;
use crate::cpu::pit;

use exceptions::IDT;
use interrupts::{prepare_default_handle, InterruptStackFrame};
use pic::CHAINED_PIC;
use pit::pit_callback;

extern "x86-interrupt" fn pit_irq0_handler(_stk: InterruptStackFrame) {
    pit_callback();
    CHAINED_PIC.lock().send_eoi(0x20);
}

pub fn setup_hw_interrupts() {
    let irq0_handle = prepare_default_handle(pit_irq0_handler);
    IDT.lock().interrupts[0] = irq0_handle;
}
