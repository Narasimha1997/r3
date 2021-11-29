extern crate log;
// Provide state management functions

// TODO: Add SSE and AVX registers

// TODO: Move to hadware based schemes like XSAVE etc

// TODO: is there a neat way?

#[derive(Clone, Debug, Default, Copy)]
#[repr(C)]
/// Stores CPU register values which can be restored at later
/// point in time, the default derivation for this struct
/// will initialize all the values to zero. Which can be used
/// for creating a new context.
pub struct CPURegistersState {
    pub rbp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl CPURegistersState {
    #[inline(always)]
    pub fn get_state() -> *const Self {
        // saves all the register states:
        let state_repr: *const Self;
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
                mov {}, rsp; 
                sub rsp, 0x400;",
                out(reg) state_repr
            );
        }

        state_repr
    }

    #[inline]
    pub fn load_state(state: &Self) {
        unsafe {
            asm!(
                "mov rsp, {};
                pop rbp; 
                pop rax; 
                pop rbx; 
                pop rcx; 
                pop rdx; 
                pop rsi; 
                pop rdi; 
                pop r8; 
                pop r9;
                pop r10;
                pop r11;
                pop r12;
                pop r13;
                pop r14;
                pop r15;
                iretq;",
                in(reg) state
            );
        }
    }
}

#[inline(never)]
/// creates a new thread by pushing some base registers to the stac
/// according to interrupt ABI specification as per IRETQ instruction.
pub fn bootstrap_kernel_thread(stack_end: u64, code: u64, cs: u16, ds: u16) {
    unsafe {
        asm!(
            "
             cli;
             push rax;
             push rsi;
             push 0x200;
             push rdx;
             push rdi;
             iretq;
            ",
            in("rdi")code,
            in("rsi")stack_end,
            in("dx") cs,
            in("ax") ds
        );
    }
}

#[naked]
pub extern "C" fn context_switch(_previous_stk: *mut u64, _next_ptr: u64) {
    unsafe {
        asm!(
            "push rbx
            push rbp
            push r12
            push r13
            push r14
            push r15
            mov [rdi], rsp
            mov rsp, rsi
            pop r15
            pop r14
            pop r13
            pop r12
            pop rbp
            pop rbx
            ret",
            options(noreturn)
        );
    }
}
