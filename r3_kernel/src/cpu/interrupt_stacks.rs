extern crate log;
use crate::cpu::segments::TaskStateSegment;

const STACK_SIZE: usize = 4096 * 16;

/// The default interrupt stack used by general interrupts.
static mut DEFAULT_INTERRUPT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

/// The default system call stack used when no stack is specified externally
/// by the thread.
static mut DEFAULT_SYSCALL_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

/// LAPIC timer uses this stack
static mut LAPIC_TIMER_INTERRPUT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

/// keyboard ps/2 uses this stack
static mut SYSTEM_KEYBOARD_INTERRUPT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

static mut PRIVILEGE_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

pub fn init_system_stacks(tss: &mut TaskStateSegment) {
    // default interrupt handler is 0th entry
    unsafe {
        tss.set_interrupt_stack(0, (&DEFAULT_INTERRUPT_STACK as *const _) as u64);
        // set the privilege stacl
        tss.set_privilege_stack(0, (&PRIVILEGE_STACK as *const _) as u64);

        // set the default system call stack
        tss.set_syscall_stack((&DEFAULT_SYSCALL_STACK as *const _) as u64);

        // set the ps/2 keyboard stack
        tss.set_interrupt_stack(2, (&SYSTEM_KEYBOARD_INTERRUPT_STACK as *const _) as u64);

        // set the LAPIC timer stack
        tss.set_interrupt_stack(3, (&LAPIC_TIMER_INTERRPUT_STACK as *const _) as u64);
    }
}
