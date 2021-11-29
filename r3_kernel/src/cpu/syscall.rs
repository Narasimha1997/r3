use crate::cpu::exceptions::IDT;
use crate::cpu::interrupts::{prepare_syscall_interrupt, InterruptStackFrame};

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
    let irq0x80_handle = prepare_syscall_interrupt(x80_handle);
    IDT.lock().interrupts_1[79] = irq0x80_handle;
}
