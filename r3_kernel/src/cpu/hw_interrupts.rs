extern crate log;

use crate::acpi::lapic::LAPICUtils;
use crate::cpu::exceptions;
use crate::cpu::interrupts;
use crate::cpu::pic;
use crate::cpu::pit;
use crate::cpu::state::CPURegistersState;
use crate::system::timer::SystemTimer;

use crate::system::scheduler::SCHEDULER;

/// hardware interrupts start from 0x20, i.e from 32
/// because of interrupt remapping.
const HARDWARE_INTERRUPTS_BASE: usize = 0x20;

/// PIT interrupt line:
const PIT_INTERRUPT_LINE: usize = 0x00;

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

#[no_mangle]
pub extern "sysv64" fn schedule_handle(state_repr: CPURegistersState) {
    // eoi:
    LAPICUtils::eoi();

    let thread_opt = SCHEDULER.lock().run_schedule(state_repr);
    if thread_opt.is_some() {
        SystemTimer::next_shot();
        thread_opt.unwrap().load_state();
    }
}

#[naked]
/// This function is called via Naked ABI: https://github.com/nox/rust-rfcs/blob/master/text/1201-naked-fns.md
/// this ABI keeps all the registers unaffected, the state of the CPU is dumped into
/// CPURegustersState type, this can be used by schedulers context switched.
/// The warning 'unsupported_naked_functions' is allowed since
/// get_state() calls assembly and is always inlined.
extern "C" fn tsc_deadline_interrupt(_stk: &mut InterruptStackFrame) {
    // as of now, this function saves the current state,
    // saves the CPU states, performs some work and enables
    // the next timer event, then loads the previously saved state
    // so execution can continue normally.
    unsafe {
        asm!(
            "push r15;
            push r14; 
            push r13;
            push r12;
            push r11;
            push r10;
            push r9;
            push r8;
            push rdi;
            push rsi;
            push rdx;
            push rcx;
            push rbx;
            push rax;
            push rbp;
            call schedule_handle",
            options(noreturn)
        );
    }
}

pub fn setup_hw_interrupts() {
    let irq0_handle = prepare_default_handle(pit_irq0_handler);
    IDT.lock().interrupts[PIT_INTERRUPT_LINE] = irq0_handle;
}

pub fn setup_post_apic_interrupts() {
    let irq0x30_handle = prepare_naked_handler(tsc_deadline_interrupt);
    IDT.lock().naked_0 = irq0x30_handle;
}
