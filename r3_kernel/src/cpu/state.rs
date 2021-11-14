// Provide state management functions

// TODO: Add SSE and AVX registers

// TODO: Move to hadware based schemes like XSAVE etc

// TODO: is there a neat way?

#[derive(Clone, Debug)]
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

    #[inline(always)]
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
