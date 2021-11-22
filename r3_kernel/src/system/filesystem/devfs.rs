extern crate alloc;
extern crate spin;

use alloc::{boxed::Box, string::String, vec::Vec};

use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug, Clone)]
pub enum DevOpErr {
    ReadError,
    WriteError,
    IOCTLError,
}

pub trait BlockDevOps {
    fn read(&self, buffer: &[u8]) -> Result<(), DevOpErr>;
    fn write(&self, buffer: &[u8]) -> Result<(), DevOpErr>;
    fn ioctl(&self, command: u8) -> Result<(), DevOpErr>;
}

pub trait CharDevOps {
    fn read(&self) -> Result<u8, DevOpErr>;
    fn write(&self) -> Result<u8, DevOpErr>;
    fn ioctl(&self) -> Result<u8, DevOpErr>;
}

pub enum DeviceType {
    BlockDevice(Box<dyn BlockDevOps>),
    CharDevice(Box<dyn CharDevOps>),
}

pub struct DevFSEntry {
    pub name: String,
    pub major: u32,
    pub minor: u32,
    pub device: DeviceType,
}

lazy_static! {
    pub static ref DEV_FS: Mutex<Vec<DevFSEntry>> = Mutex::new(Vec::new());
}

/// a driver that handles all these operations:
pub struct DevFSDriver {}
