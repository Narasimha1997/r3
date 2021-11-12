use crate::mm::VirtualAddress;

use core::ptr;

#[derive(Clone, Copy)]
/// Represents a memory address from where data
/// can be read or written, it is similar to h/w ports in cpu::io
pub struct MemoryIO {
    pub address: VirtualAddress,
    pub read_only: bool,
}

impl MemoryIO {
    pub fn new(address: VirtualAddress, read_only: bool) -> Self {
        MemoryIO { address, read_only }
    }

    pub fn read_u8(&self) -> u8 {
        unsafe { ptr::read_volatile(self.address.as_u64() as *const u8) }
    }

    pub fn write_u8(&self, value: u8) {
        unsafe { ptr::write_volatile(self.address.as_u64() as *mut u8, value) }
    }

    pub fn read_u16(&self) -> u16 {
        unsafe { ptr::read_volatile(self.address.as_u64() as *const u16) }
    }

    pub fn write_u16(&self, value: u16) {
        unsafe { ptr::write_volatile(self.address.as_u64() as *mut u16, value) }
    }

    pub fn read_u32(&self) -> u32 {
        unsafe { ptr::read_volatile(self.address.as_u64() as *const u32) }
    }

    pub fn write_u32(&self, value: u32) {
        unsafe { ptr::write_volatile(self.address.as_u64() as *mut u32, value) }
    }

    pub fn read_u64(&self) -> u64 {
        unsafe { ptr::read_volatile(self.address.as_u64() as *const u64) }
    }

    pub fn write_64(&self, value: u64) {
        unsafe { ptr::write_volatile(self.address.as_u64() as *mut u64, value) }
    }
}
