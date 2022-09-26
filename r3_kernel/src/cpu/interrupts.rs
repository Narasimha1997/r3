extern crate bit_field;

use core::arch::asm;

use crate::cpu::mmu::PageFaultExceptionTypes;
use crate::cpu::segments;

use bit_field::BitField;
use core::fmt;
use core::mem;

use segments::SegmentRegister;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

impl fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut format_string = f.debug_struct("Exception Info");
        format_string.field("instruction_pointer", &self.instruction_pointer);
        format_string.field("code_segment", &self.code_segment);
        format_string.field("cpu_flags", &self.cpu_flags);
        format_string.field("stack_pointer", &self.stack_pointer);
        format_string.field("stack_segment", &self.stack_segment);

        format_string.finish()
    }
}

const DEFAULT_INTERRUPT_OPTION_BITS: u16 = 0b1110_0000_0000;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptDescriptorEntry {
    pointer_low: u16,
    gdt_selector: u16,
    options: u16,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32
}

impl fmt::Debug for InterruptDescriptorEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut format_string = f.debug_struct("InterruptDescriptorEntry");
        format_string.field("handler_addr", &self.get_handler_addr());
        format_string.field("options", &self.options);
        format_string.field("gdt_selector", &self.gdt_selector);
        format_string.finish()
    }
}

impl InterruptDescriptorEntry {
    #[inline]
    pub fn empty() -> Self {
        InterruptDescriptorEntry {
            pointer_low: 0,
            pointer_high: 0,
            pointer_middle: 0,
            options: DEFAULT_INTERRUPT_OPTION_BITS,
            gdt_selector: 0,
            reserved: 0
        }
    }

    #[inline]
    fn read_cs(&self) -> u16 {
        SegmentRegister::CS.get()
    }

    #[inline]
    fn set_pointers(&mut self, addr: u64) {
        self.pointer_low = (addr & 0xffff) as u16;
        self.pointer_middle = ((addr >> 16) & 0xffff) as u16;
        self.pointer_high = ((addr >> 32) & 0xffffffff) as u32;
    }

    #[inline]
    pub fn get_handler_addr(&self) -> u64 {
        let low = self.pointer_low as u64;
        let middle = (self.pointer_middle as u64) << 16;
        let high = (self.pointer_high as u64) << 32;

        low | high | middle
    }

    #[inline]
    pub fn set_handler(&mut self, handler_address: u64) {
        // set high, low and middle pointers
        self.set_pointers(handler_address);

        // get the cs register:
        self.gdt_selector = self.read_cs();
        self.options.set_bit(15, true);
    }

    #[inline]
    pub fn set_stack_index(&mut self, stack_index: u16) {
        self.options.set_bits(0..3, stack_index + 1);
    }

    #[inline]
    pub fn set_privilege_level(&mut self, dpl: segments::PrivilegeLevel) {
        self.options.set_bits(13..15, dpl as u16);
    }
}

// error handler function:

// A generic handler function
pub type DefaultHandlerFunction = extern "x86-interrupt" fn(InterruptStackFrame);

// A handler function with error code
pub type HandlerFunctionWithErr = extern "x86-interrupt" fn(InterruptStackFrame, u64);

// A handler function that handles unrecoverable errors:
pub type DefaultHandlerFuncNoReturn = extern "x86-interrupt" fn(InterruptStackFrame) -> !;

// A handler function that handles unrecoverable errors with error code:
pub type HandlerFuncNoReturnWithErr = extern "x86-interrupt" fn(InterruptStackFrame, u64) -> !;

pub type PageFaultHandlerType =
    extern "x86-interrupt" fn(InterruptStackFrame, PageFaultExceptionTypes) -> !;

pub type NakedHandlerType = extern "C" fn(&mut InterruptStackFrame);

pub type Sysv64HandlerType = extern "sysv64" fn(&mut InterruptStackFrame);

// pointer struct which points to the IDT table:
#[repr(C, packed)]
pub struct IDTPointer {
    pub size_limit: u16,
    pub base_addr: u64,
}

#[derive(Clone, Debug)]
#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    pub interrupts: [InterruptDescriptorEntry; 256]
}

impl InterruptDescriptorTable {
    pub fn empty() -> Self {
        InterruptDescriptorTable {
            interrupts: [InterruptDescriptorEntry::empty(); 256]
        }
    }

    pub fn as_pointer(&self) -> IDTPointer {
        IDTPointer {
            base_addr: (self as *const _) as u64,
            size_limit: (mem::size_of::<Self>() - 1) as u16,
        }
    }

    pub fn load_into_cpu(&self) {
        let pointer = self.as_pointer();
        unsafe {
            asm!(
                "lidt [{}]", in(reg) &pointer,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}

pub fn prepare_default_handle(
    func: DefaultHandlerFunction,
    stack_index: u16,
) -> InterruptDescriptorEntry{
    let handle_addr = func as u64;
    let mut idt_entry = InterruptDescriptorEntry::empty();
    idt_entry.set_handler(handle_addr);
    if stack_index > 0 {
        idt_entry.set_stack_index(stack_index);
    }
    return idt_entry;
}

pub fn prepare_no_ret_error_code_handle(
    func: HandlerFuncNoReturnWithErr,
) -> InterruptDescriptorEntry {
    let handle_addr = func as u64;
    let mut idt_entry = InterruptDescriptorEntry::empty();
    idt_entry.set_handler(handle_addr);
    return idt_entry;
}

pub fn prepare_page_fault_handler(
    func: PageFaultHandlerType,
) -> InterruptDescriptorEntry {
    let handle_addr = func as u64;
    let mut idt_entry = InterruptDescriptorEntry::empty();
    idt_entry.set_handler(handle_addr);
    return idt_entry;
}

pub fn prepare_naked_handler(
    func: NakedHandlerType,
    stack_index: u16,
) -> InterruptDescriptorEntry {
    let handle_addr = func as u64;
    let mut idt_entry = InterruptDescriptorEntry::empty();
    idt_entry.set_handler(handle_addr);
    idt_entry.set_stack_index(stack_index);
    return idt_entry;
}

pub fn prepare_error_code_handle(
    func: HandlerFunctionWithErr,
) -> InterruptDescriptorEntry {
    let handle_addr = func as u64;
    let mut idt_entry = InterruptDescriptorEntry::empty();
    idt_entry.set_handler(handle_addr);
    return idt_entry;
}

pub fn prepare_syscall_interrupt(
    func: Sysv64HandlerType,
    stack_index: u16,
) -> InterruptDescriptorEntry {
    let handle_addr = func as u64;
    let mut idt_entry = InterruptDescriptorEntry::empty();
    idt_entry.set_handler(handle_addr);
    idt_entry.set_privilege_level(segments::PrivilegeLevel::Ring3);
    if stack_index > 0 {
        idt_entry.set_stack_index(stack_index);
    }
    return idt_entry;
}
