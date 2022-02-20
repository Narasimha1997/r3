extern crate log;

use crate::acpi::lapic::LAPICUtils;
use crate::cpu::exceptions;
use crate::cpu::interrupts;
use crate::cpu::pic;
use crate::cpu::pit;
use crate::drivers::keyboard;
use crate::system::net::iface::network_interrupt_handler;

use core::sync::atomic::{AtomicUsize, Ordering};

#[allow(unused_imports)]
// unused because this is called from assembly
use crate::system::tasking::schedule_handle;

pub const HARDWARE_INTERRUPTS_BASE: usize = 0x30;

/// hardware interrupts start from 0x20, i.e from 32
/// because of interrupt remapping.
pub const LEGACY_HARDWARE_INTERRUPTS_BASE: usize = 0x20;

/// PIT interrupt line:
const PIT_INTERRUPT_LINE: usize = 0x00;

/// Maximum interrupts possible on x86
const MAX_ARCH_INTERRUPTS: usize = 256;

/// ATA interrupt line - PRIMARY master:
const ATA_PRIMARY_INTERRIUPT_LINE: usize = 0x0E;

/// ATA interrupt line - SECONDARY slave:
const ATA_SECONDARY_INTERRUPT_LINE: usize = 0x0F;

/// Timeshot interrupt line
const TIMESHOT_INTERRUPT_LINE: usize = 0x20;

/// Keyboard controller interrupt line:
const KEYBOARD_INTERRUPT_LINE: usize = 0x01;

static NETWORK_INTERRUPT_NO: AtomicUsize = AtomicUsize::new(0);

use exceptions::IDT;
use interrupts::{prepare_default_handle, prepare_naked_handler, InterruptStackFrame};
use pic::CHAINED_PIC;
use pit::pit_callback;

#[inline]
fn ack_hw_interrupt(interrupt_no: u8) {
    // PIC mode?
    let ch_lock = CHAINED_PIC.lock();
    ch_lock.send_eoi(interrupt_no);
}

extern "x86-interrupt" fn pit_irq0_handler_legacy(_stk: InterruptStackFrame) {
    pit_callback();
    // 0th line is PIT
    // ack_hw_interrupt((LEGACY_HARDWARE_INTERRUPTS_BASE + PIT_INTERRUPT_LINE) as u8);
    ack_hw_interrupt((LEGACY_HARDWARE_INTERRUPTS_BASE + PIT_INTERRUPT_LINE) as u8);
}

extern "x86-interrupt" fn kbd_irq1_handler(_stk: InterruptStackFrame) {
    keyboard::PC_KEYBOARD.lock().read_key();
    LAPICUtils::eoi();
}

extern "x86-interrupt" fn ata_irq14_handler(_stk: InterruptStackFrame) {
    LAPICUtils::eoi();
}

extern "x86-interrupt" fn ata_irq15_handler(_stk: InterruptStackFrame) {
    LAPICUtils::eoi();
}

extern "x86-interrupt" fn net_interrupt_wrapper(_stk: InterruptStackFrame) {
    log::debug!("network interrupt!");
    network_interrupt_handler();
    // LAPICUtils::eoi();
}

extern "x86-interrupt" fn no_interrupt_handler(stk: InterruptStackFrame) {
    log::debug!("Unhandled interrupt received: {:?}", stk);
}

#[naked]
extern "C" fn tsc_deadline_interrupt(_stk: &mut InterruptStackFrame) {
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
    // PIT legacy timer
    let irq0x00_handle = prepare_default_handle(pit_irq0_handler_legacy, 0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + PIT_INTERRUPT_LINE] = irq0x00_handle;

    // fill remaining interrupts with legacy handler
    for idx in 1..(MAX_ARCH_INTERRUPTS - LEGACY_HARDWARE_INTERRUPTS_BASE) {
        // catch an unhandled hardware interrupt
        let irq_no_entry = prepare_default_handle(no_interrupt_handler, 0);
        IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + idx] = irq_no_entry;
    }

    // ATA 14 primary
    let irq0x0e_handle = prepare_default_handle(ata_irq14_handler, 0);
    IDT.lock().interrupts[HARDWARE_INTERRUPTS_BASE + ATA_PRIMARY_INTERRIUPT_LINE] = irq0x0e_handle;

    // ATA 15 secondary
    let irq0x0f_handle = prepare_default_handle(ata_irq15_handler, 0);
    IDT.lock().interrupts[HARDWARE_INTERRUPTS_BASE + ATA_SECONDARY_INTERRUPT_LINE] = irq0x0f_handle;
}

pub fn setup_post_apic_interrupts() {
    let irq0x50_handle = prepare_naked_handler(tsc_deadline_interrupt, 3);
    IDT.lock().interrupts[HARDWARE_INTERRUPTS_BASE + TIMESHOT_INTERRUPT_LINE] = irq0x50_handle;

    let irq0x01_handle = prepare_default_handle(kbd_irq1_handler, 2);
    IDT.lock().interrupts[HARDWARE_INTERRUPTS_BASE + KEYBOARD_INTERRUPT_LINE] = irq0x01_handle;
}

pub fn register_network_interrupt(int_no: usize) {
    NETWORK_INTERRUPT_NO.store(int_no, Ordering::Relaxed);
    let irq_handler = prepare_default_handle(net_interrupt_wrapper, 4);
    IDT.lock().interrupts[HARDWARE_INTERRUPTS_BASE + int_no] = irq_handler;
}
