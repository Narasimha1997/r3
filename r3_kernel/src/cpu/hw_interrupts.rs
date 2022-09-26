extern crate log;

use core::arch::asm;

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

macro_rules! prepare_no_irq_handler {
    ($name:ident, $ist:expr) => {{
        extern "x86-interrupt" fn wrapper(_: InterruptStackFrame) {
            ($name)($ist);
        }
        let handler = prepare_default_handle(wrapper, 0);
        handler
    }};
}

#[inline]
fn ack_hw_interrupt(interrupt_no: u8) {
    // PIC mode?
    let pit_lock = CHAINED_PIC.lock();
    if pit_lock.is_enabled {
        pit_lock.send_eoi(interrupt_no);
    } else {
        LAPICUtils::eoi();
    }
}

extern "x86-interrupt" fn pit_irq0_handler_legacy(_stk: InterruptStackFrame) {
    pit_callback();
    // 0th line is PIT
    // ack_hw_interrupt((LEGACY_HARDWARE_INTERRUPTS_BASE + PIT_INTERRUPT_LINE) as u8);
    log::debug!("PIT interrupt");
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
    network_interrupt_handler();
    LAPICUtils::eoi();
}

fn no_irq_fn(irq_no: usize) {
    log::debug!("dev interrupt {:x}", irq_no);
    LAPICUtils::eoi();
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
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1] =
        prepare_no_irq_handler!(no_irq_fn, 0x21);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2] =
        prepare_no_irq_handler!(no_irq_fn, 0x22);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3] =
        prepare_no_irq_handler!(no_irq_fn, 0x23);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4] =
        prepare_no_irq_handler!(no_irq_fn, 0x24);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5] =
        prepare_no_irq_handler!(no_irq_fn, 0x25);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6] =
        prepare_no_irq_handler!(no_irq_fn, 0x26);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7] =
        prepare_no_irq_handler!(no_irq_fn, 0x27);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8] =
        prepare_no_irq_handler!(no_irq_fn, 0x28);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9] =
        prepare_no_irq_handler!(no_irq_fn, 0x29);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa] =
        prepare_no_irq_handler!(no_irq_fn, 0x2a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb] =
        prepare_no_irq_handler!(no_irq_fn, 0x2b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc] =
        prepare_no_irq_handler!(no_irq_fn, 0x2c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd] =
        prepare_no_irq_handler!(no_irq_fn, 0x2d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xe] =
        prepare_no_irq_handler!(no_irq_fn, 0x2e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xf] =
        prepare_no_irq_handler!(no_irq_fn, 0x2f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x10] =
        prepare_no_irq_handler!(no_irq_fn, 0x30);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x11] =
        prepare_no_irq_handler!(no_irq_fn, 0x31);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x12] =
        prepare_no_irq_handler!(no_irq_fn, 0x32);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x13] =
        prepare_no_irq_handler!(no_irq_fn, 0x33);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x14] =
        prepare_no_irq_handler!(no_irq_fn, 0x34);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x15] =
        prepare_no_irq_handler!(no_irq_fn, 0x35);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x16] =
        prepare_no_irq_handler!(no_irq_fn, 0x36);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x17] =
        prepare_no_irq_handler!(no_irq_fn, 0x37);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x18] =
        prepare_no_irq_handler!(no_irq_fn, 0x38);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x19] =
        prepare_no_irq_handler!(no_irq_fn, 0x39);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1a] =
        prepare_no_irq_handler!(no_irq_fn, 0x3a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1b] =
        prepare_no_irq_handler!(no_irq_fn, 0x3b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1c] =
        prepare_no_irq_handler!(no_irq_fn, 0x3c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1d] =
        prepare_no_irq_handler!(no_irq_fn, 0x3d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1e] =
        prepare_no_irq_handler!(no_irq_fn, 0x3e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x1f] =
        prepare_no_irq_handler!(no_irq_fn, 0x3f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x20] =
        prepare_no_irq_handler!(no_irq_fn, 0x40);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x21] =
        prepare_no_irq_handler!(no_irq_fn, 0x41);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x22] =
        prepare_no_irq_handler!(no_irq_fn, 0x42);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x23] =
        prepare_no_irq_handler!(no_irq_fn, 0x43);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x24] =
        prepare_no_irq_handler!(no_irq_fn, 0x44);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x25] =
        prepare_no_irq_handler!(no_irq_fn, 0x45);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x26] =
        prepare_no_irq_handler!(no_irq_fn, 0x46);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x27] =
        prepare_no_irq_handler!(no_irq_fn, 0x47);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x28] =
        prepare_no_irq_handler!(no_irq_fn, 0x48);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x29] =
        prepare_no_irq_handler!(no_irq_fn, 0x49);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2a] =
        prepare_no_irq_handler!(no_irq_fn, 0x4a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2b] =
        prepare_no_irq_handler!(no_irq_fn, 0x4b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2c] =
        prepare_no_irq_handler!(no_irq_fn, 0x4c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2d] =
        prepare_no_irq_handler!(no_irq_fn, 0x4d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2e] =
        prepare_no_irq_handler!(no_irq_fn, 0x4e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x2f] =
        prepare_no_irq_handler!(no_irq_fn, 0x4f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x30] =
        prepare_no_irq_handler!(no_irq_fn, 0x50);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x31] =
        prepare_no_irq_handler!(no_irq_fn, 0x51);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x32] =
        prepare_no_irq_handler!(no_irq_fn, 0x52);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x33] =
        prepare_no_irq_handler!(no_irq_fn, 0x53);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x34] =
        prepare_no_irq_handler!(no_irq_fn, 0x54);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x35] =
        prepare_no_irq_handler!(no_irq_fn, 0x55);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x36] =
        prepare_no_irq_handler!(no_irq_fn, 0x56);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x37] =
        prepare_no_irq_handler!(no_irq_fn, 0x57);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x38] =
        prepare_no_irq_handler!(no_irq_fn, 0x58);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x39] =
        prepare_no_irq_handler!(no_irq_fn, 0x59);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3a] =
        prepare_no_irq_handler!(no_irq_fn, 0x5a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3b] =
        prepare_no_irq_handler!(no_irq_fn, 0x5b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3c] =
        prepare_no_irq_handler!(no_irq_fn, 0x5c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3d] =
        prepare_no_irq_handler!(no_irq_fn, 0x5d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3e] =
        prepare_no_irq_handler!(no_irq_fn, 0x5e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x3f] =
        prepare_no_irq_handler!(no_irq_fn, 0x5f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x40] =
        prepare_no_irq_handler!(no_irq_fn, 0x60);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x41] =
        prepare_no_irq_handler!(no_irq_fn, 0x61);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x42] =
        prepare_no_irq_handler!(no_irq_fn, 0x62);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x43] =
        prepare_no_irq_handler!(no_irq_fn, 0x63);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x44] =
        prepare_no_irq_handler!(no_irq_fn, 0x64);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x45] =
        prepare_no_irq_handler!(no_irq_fn, 0x65);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x46] =
        prepare_no_irq_handler!(no_irq_fn, 0x66);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x47] =
        prepare_no_irq_handler!(no_irq_fn, 0x67);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x48] =
        prepare_no_irq_handler!(no_irq_fn, 0x68);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x49] =
        prepare_no_irq_handler!(no_irq_fn, 0x69);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4a] =
        prepare_no_irq_handler!(no_irq_fn, 0x6a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4b] =
        prepare_no_irq_handler!(no_irq_fn, 0x6b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4c] =
        prepare_no_irq_handler!(no_irq_fn, 0x6c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4d] =
        prepare_no_irq_handler!(no_irq_fn, 0x6d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4e] =
        prepare_no_irq_handler!(no_irq_fn, 0x6e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x4f] =
        prepare_no_irq_handler!(no_irq_fn, 0x6f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x50] =
        prepare_no_irq_handler!(no_irq_fn, 0x70);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x51] =
        prepare_no_irq_handler!(no_irq_fn, 0x71);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x52] =
        prepare_no_irq_handler!(no_irq_fn, 0x72);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x53] =
        prepare_no_irq_handler!(no_irq_fn, 0x73);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x54] =
        prepare_no_irq_handler!(no_irq_fn, 0x74);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x55] =
        prepare_no_irq_handler!(no_irq_fn, 0x75);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x56] =
        prepare_no_irq_handler!(no_irq_fn, 0x76);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x57] =
        prepare_no_irq_handler!(no_irq_fn, 0x77);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x58] =
        prepare_no_irq_handler!(no_irq_fn, 0x78);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x59] =
        prepare_no_irq_handler!(no_irq_fn, 0x79);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5a] =
        prepare_no_irq_handler!(no_irq_fn, 0x7a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5b] =
        prepare_no_irq_handler!(no_irq_fn, 0x7b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5c] =
        prepare_no_irq_handler!(no_irq_fn, 0x7c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5d] =
        prepare_no_irq_handler!(no_irq_fn, 0x7d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5e] =
        prepare_no_irq_handler!(no_irq_fn, 0x7e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x5f] =
        prepare_no_irq_handler!(no_irq_fn, 0x7f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x60] =
        prepare_no_irq_handler!(no_irq_fn, 0x80);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x61] =
        prepare_no_irq_handler!(no_irq_fn, 0x81);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x62] =
        prepare_no_irq_handler!(no_irq_fn, 0x82);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x63] =
        prepare_no_irq_handler!(no_irq_fn, 0x83);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x64] =
        prepare_no_irq_handler!(no_irq_fn, 0x84);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x65] =
        prepare_no_irq_handler!(no_irq_fn, 0x85);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x66] =
        prepare_no_irq_handler!(no_irq_fn, 0x86);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x67] =
        prepare_no_irq_handler!(no_irq_fn, 0x87);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x68] =
        prepare_no_irq_handler!(no_irq_fn, 0x88);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x69] =
        prepare_no_irq_handler!(no_irq_fn, 0x89);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6a] =
        prepare_no_irq_handler!(no_irq_fn, 0x8a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6b] =
        prepare_no_irq_handler!(no_irq_fn, 0x8b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6c] =
        prepare_no_irq_handler!(no_irq_fn, 0x8c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6d] =
        prepare_no_irq_handler!(no_irq_fn, 0x8d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6e] =
        prepare_no_irq_handler!(no_irq_fn, 0x8e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x6f] =
        prepare_no_irq_handler!(no_irq_fn, 0x8f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x70] =
        prepare_no_irq_handler!(no_irq_fn, 0x90);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x71] =
        prepare_no_irq_handler!(no_irq_fn, 0x91);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x72] =
        prepare_no_irq_handler!(no_irq_fn, 0x92);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x73] =
        prepare_no_irq_handler!(no_irq_fn, 0x93);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x74] =
        prepare_no_irq_handler!(no_irq_fn, 0x94);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x75] =
        prepare_no_irq_handler!(no_irq_fn, 0x95);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x76] =
        prepare_no_irq_handler!(no_irq_fn, 0x96);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x77] =
        prepare_no_irq_handler!(no_irq_fn, 0x97);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x78] =
        prepare_no_irq_handler!(no_irq_fn, 0x98);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x79] =
        prepare_no_irq_handler!(no_irq_fn, 0x99);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7a] =
        prepare_no_irq_handler!(no_irq_fn, 0x9a);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7b] =
        prepare_no_irq_handler!(no_irq_fn, 0x9b);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7c] =
        prepare_no_irq_handler!(no_irq_fn, 0x9c);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7d] =
        prepare_no_irq_handler!(no_irq_fn, 0x9d);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7e] =
        prepare_no_irq_handler!(no_irq_fn, 0x9e);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x7f] =
        prepare_no_irq_handler!(no_irq_fn, 0x9f);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x80] =
        prepare_no_irq_handler!(no_irq_fn, 0xa0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x81] =
        prepare_no_irq_handler!(no_irq_fn, 0xa1);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x82] =
        prepare_no_irq_handler!(no_irq_fn, 0xa2);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x83] =
        prepare_no_irq_handler!(no_irq_fn, 0xa3);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x84] =
        prepare_no_irq_handler!(no_irq_fn, 0xa4);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x85] =
        prepare_no_irq_handler!(no_irq_fn, 0xa5);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x86] =
        prepare_no_irq_handler!(no_irq_fn, 0xa6);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x87] =
        prepare_no_irq_handler!(no_irq_fn, 0xa7);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x88] =
        prepare_no_irq_handler!(no_irq_fn, 0xa8);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x89] =
        prepare_no_irq_handler!(no_irq_fn, 0xa9);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8a] =
        prepare_no_irq_handler!(no_irq_fn, 0xaa);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8b] =
        prepare_no_irq_handler!(no_irq_fn, 0xab);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8c] =
        prepare_no_irq_handler!(no_irq_fn, 0xac);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8d] =
        prepare_no_irq_handler!(no_irq_fn, 0xad);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8e] =
        prepare_no_irq_handler!(no_irq_fn, 0xae);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x8f] =
        prepare_no_irq_handler!(no_irq_fn, 0xaf);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x90] =
        prepare_no_irq_handler!(no_irq_fn, 0xb0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x91] =
        prepare_no_irq_handler!(no_irq_fn, 0xb1);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x92] =
        prepare_no_irq_handler!(no_irq_fn, 0xb2);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x93] =
        prepare_no_irq_handler!(no_irq_fn, 0xb3);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x94] =
        prepare_no_irq_handler!(no_irq_fn, 0xb4);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x95] =
        prepare_no_irq_handler!(no_irq_fn, 0xb5);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x96] =
        prepare_no_irq_handler!(no_irq_fn, 0xb6);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x97] =
        prepare_no_irq_handler!(no_irq_fn, 0xb7);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x98] =
        prepare_no_irq_handler!(no_irq_fn, 0xb8);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x99] =
        prepare_no_irq_handler!(no_irq_fn, 0xb9);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9a] =
        prepare_no_irq_handler!(no_irq_fn, 0xba);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9b] =
        prepare_no_irq_handler!(no_irq_fn, 0xbb);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9c] =
        prepare_no_irq_handler!(no_irq_fn, 0xbc);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9d] =
        prepare_no_irq_handler!(no_irq_fn, 0xbd);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9e] =
        prepare_no_irq_handler!(no_irq_fn, 0xbe);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0x9f] =
        prepare_no_irq_handler!(no_irq_fn, 0xbf);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa0] =
        prepare_no_irq_handler!(no_irq_fn, 0xc0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa1] =
        prepare_no_irq_handler!(no_irq_fn, 0xc1);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa2] =
        prepare_no_irq_handler!(no_irq_fn, 0xc2);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa3] =
        prepare_no_irq_handler!(no_irq_fn, 0xc3);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa4] =
        prepare_no_irq_handler!(no_irq_fn, 0xc4);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa5] =
        prepare_no_irq_handler!(no_irq_fn, 0xc5);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa6] =
        prepare_no_irq_handler!(no_irq_fn, 0xc6);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa7] =
        prepare_no_irq_handler!(no_irq_fn, 0xc7);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa8] =
        prepare_no_irq_handler!(no_irq_fn, 0xc8);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xa9] =
        prepare_no_irq_handler!(no_irq_fn, 0xc9);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xaa] =
        prepare_no_irq_handler!(no_irq_fn, 0xca);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xab] =
        prepare_no_irq_handler!(no_irq_fn, 0xcb);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xac] =
        prepare_no_irq_handler!(no_irq_fn, 0xcc);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xad] =
        prepare_no_irq_handler!(no_irq_fn, 0xcd);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xae] =
        prepare_no_irq_handler!(no_irq_fn, 0xce);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xaf] =
        prepare_no_irq_handler!(no_irq_fn, 0xcf);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb0] =
        prepare_no_irq_handler!(no_irq_fn, 0xd0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb1] =
        prepare_no_irq_handler!(no_irq_fn, 0xd1);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb2] =
        prepare_no_irq_handler!(no_irq_fn, 0xd2);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb3] =
        prepare_no_irq_handler!(no_irq_fn, 0xd3);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb4] =
        prepare_no_irq_handler!(no_irq_fn, 0xd4);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb5] =
        prepare_no_irq_handler!(no_irq_fn, 0xd5);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb6] =
        prepare_no_irq_handler!(no_irq_fn, 0xd6);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb7] =
        prepare_no_irq_handler!(no_irq_fn, 0xd7);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb8] =
        prepare_no_irq_handler!(no_irq_fn, 0xd8);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xb9] =
        prepare_no_irq_handler!(no_irq_fn, 0xd9);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xba] =
        prepare_no_irq_handler!(no_irq_fn, 0xda);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xbb] =
        prepare_no_irq_handler!(no_irq_fn, 0xdb);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xbc] =
        prepare_no_irq_handler!(no_irq_fn, 0xdc);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xbd] =
        prepare_no_irq_handler!(no_irq_fn, 0xdd);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xbe] =
        prepare_no_irq_handler!(no_irq_fn, 0xde);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xbf] =
        prepare_no_irq_handler!(no_irq_fn, 0xdf);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc0] =
        prepare_no_irq_handler!(no_irq_fn, 0xe0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc1] =
        prepare_no_irq_handler!(no_irq_fn, 0xe1);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc2] =
        prepare_no_irq_handler!(no_irq_fn, 0xe2);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc3] =
        prepare_no_irq_handler!(no_irq_fn, 0xe3);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc4] =
        prepare_no_irq_handler!(no_irq_fn, 0xe4);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc5] =
        prepare_no_irq_handler!(no_irq_fn, 0xe5);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc6] =
        prepare_no_irq_handler!(no_irq_fn, 0xe6);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc7] =
        prepare_no_irq_handler!(no_irq_fn, 0xe7);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc8] =
        prepare_no_irq_handler!(no_irq_fn, 0xe8);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xc9] =
        prepare_no_irq_handler!(no_irq_fn, 0xe9);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xca] =
        prepare_no_irq_handler!(no_irq_fn, 0xea);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xcb] =
        prepare_no_irq_handler!(no_irq_fn, 0xeb);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xcc] =
        prepare_no_irq_handler!(no_irq_fn, 0xec);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xcd] =
        prepare_no_irq_handler!(no_irq_fn, 0xed);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xce] =
        prepare_no_irq_handler!(no_irq_fn, 0xee);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xcf] =
        prepare_no_irq_handler!(no_irq_fn, 0xef);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd0] =
        prepare_no_irq_handler!(no_irq_fn, 0xf0);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd1] =
        prepare_no_irq_handler!(no_irq_fn, 0xf1);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd2] =
        prepare_no_irq_handler!(no_irq_fn, 0xf2);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd3] =
        prepare_no_irq_handler!(no_irq_fn, 0xf3);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd4] =
        prepare_no_irq_handler!(no_irq_fn, 0xf4);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd5] =
        prepare_no_irq_handler!(no_irq_fn, 0xf5);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd6] =
        prepare_no_irq_handler!(no_irq_fn, 0xf6);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd7] =
        prepare_no_irq_handler!(no_irq_fn, 0xf7);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd8] =
        prepare_no_irq_handler!(no_irq_fn, 0xf8);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xd9] =
        prepare_no_irq_handler!(no_irq_fn, 0xf9);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xda] =
        prepare_no_irq_handler!(no_irq_fn, 0xfa);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xdb] =
        prepare_no_irq_handler!(no_irq_fn, 0xfb);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xdc] =
        prepare_no_irq_handler!(no_irq_fn, 0xfc);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xdd] =
        prepare_no_irq_handler!(no_irq_fn, 0xfd);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xde] =
        prepare_no_irq_handler!(no_irq_fn, 0xfe);
    IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + 0xdf] =
        prepare_no_irq_handler!(no_irq_fn, 0xff);

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
