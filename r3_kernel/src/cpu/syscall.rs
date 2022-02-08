use crate::cpu::exceptions::IDT;
use crate::cpu::interrupt_stacks::load_default_syscall_stack;
use crate::cpu::interrupts::{prepare_syscall_interrupt, InterruptStackFrame};
use crate::cpu::segments::KERNEL_TSS;

#[allow(unused_imports)]
// called by assembly
use crate::system::abi::syscall_handler;

macro_rules! save_syscall_registers {
    () => {
        r#"
        push rax;
        push rcx;
        push rdx;
        push rsi;
        push rdi;
        push r8;
        push r9;
        push r10;
        push r11;
        "#
    };
}

macro_rules! restore_syscall_registers {
    () => {
        r#"
        pop r11;
        pop r10;
        pop r9;
        pop r8;
        pop rdi;
        pop rsi;
        pop rdx;
        pop rcx;
        pop rax;
        iretq;
        "#
    };
}

#[naked]
/// This handle will be called on int 80 soft interrupt
/// line.
pub extern "sysv64" fn x80_handle(_stk: &mut InterruptStackFrame) {
    unsafe {
        asm!(
            save_syscall_registers!(),
            "mov rsi, rsp",
            "mov rdi, rsp",
            "add rdi, 72",
            "call syscall_handler",
            restore_syscall_registers!(),
            options(noreturn)
        )
    }
}

pub fn setup_syscall_interrupt() {
    let irq0x80_handle = prepare_syscall_interrupt(x80_handle, 1);
    IDT.lock().interrupts[0x80] = irq0x80_handle;
}

pub fn set_syscall_stack(addr: u64) {
    KERNEL_TSS.lock().set_syscall_stack(addr);
}

pub fn set_default_syscall_stack() {
    load_default_syscall_stack();
}
