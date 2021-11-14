extern crate log;

use crate::acpi::lapic;
use crate::cpu::exceptions;
use crate::cpu::interrupts;
use crate::cpu::pic;
use crate::cpu::pit;
use crate::system::timer::SystemTimer;

/// hardware interrupts start from 0x20, i.e from 32
/// because of interrupt remapping.
const HARDWARE_INTERRUPTS_BASE: usize = 0x20;

/// PIT interrupt line:
const PIT_INTERRUPT_LINE: usize = 0x00;

/// LAPIC Timer interrupt line
const LAPIC_TIMER_INTERRUPT: usize = 0x10;

use exceptions::IDT;
use interrupts::{prepare_default_handle, prepare_naked_handler, InterruptStackFrame};
use pic::CHAINED_PIC;
use pit::pit_callback;

extern "x86-interrupt" fn pit_irq0_handler(_stk: InterruptStackFrame) {
    pit_callback();
    // 0th line is PIT
    CHAINED_PIC
        .lock()
        .send_eoi((HARDWARE_INTERRUPTS_BASE + PIT_INTERRUPT_LINE) as u8);
}

#[naked]
extern "C" fn tsc_deadline_interrupt(_stk: InterruptStackFrame) {
    lapic::LAPICUtils::eoi();

    SystemTimer::post_shot();
}

pub fn setup_hw_interrupts() {
    let irq0_handle = prepare_default_handle(pit_irq0_handler);
    IDT.lock().interrupts[PIT_INTERRUPT_LINE] = irq0_handle;
}

pub fn setup_post_apic_interrupts() {
    let irq0x30_handle = prepare_naked_handler(tsc_deadline_interrupt);
    IDT.lock().naked_0 = irq0x30_handle;
}
