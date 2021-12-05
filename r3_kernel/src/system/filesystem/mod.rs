extern crate alloc;
extern crate bitflags;

pub mod devfs;
pub mod paths;
pub mod ustar;
pub mod vfs;

use bitflags::bitflags;

bitflags! {
    pub struct POSIXOpenFlags: u32 {
        const O_RDONLY = 0o0;
        const O_WRONLY = 0o1;
        const O_RDWR = 0o2;
        const O_CREAT = 0o100;
        const O_EXCL = 0o200;
        const O_NOCTTY = 0o400;
        const O_TRUNC = 0o1000;
        const O_APPEND = 0o2000;
        const O_NONBLOCK = 0o4000;
        const O_DIRECTORY = 0o200000;
        const O_CLOEXEC  = 0o2000000;
    }
}

pub struct FileMode(u32);

#[derive(Debug, Clone)]
pub enum MountInfo {
    DevFS(devfs::DevFSDriver),
    MemFS,
    BlockFS,
    TarFS(ustar::TarFSDriver),
}

#[derive(Debug, Clone)]
/// Represents an entry node in VFS tree
pub enum FileDescriptor {
    DevFSNode(devfs::DevFSDescriptor),
    Ext2Node,
    TarFSNode(ustar::TarFileDescriptor),
    Empty,
}

#[derive(Debug, Clone)]
pub struct Direntry {}

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum FSError {
    NotYetImplemented,
    InvalidOperation,
    NotFound,
    AlreadyExist,
    IllegalPath,
    Busy,
    DeviceNotFound,
    InvalidSeek,
    AlignmentError,
    IOError,
}

/// Represents the operations performed on File-System
pub trait FSOps {
    fn open(&mut self, _path: &str, _flags: u32) -> Result<FileDescriptor, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn close(&self, _fd: &FileDescriptor) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
}

/// operations on file-descriptor
pub trait FDOps {
    fn read(&self, _fd: &mut FileDescriptor, _buffer: &mut [u8]) -> Result<usize, FSError> {
        Err(FSError::NotYetImplemented)
    }
    fn write(&self, _fd: &mut FileDescriptor, _buffer: &[u8]) -> Result<usize, FSError> {
        Err(FSError::NotYetImplemented)
    }
    fn ioctl(&self, _fd: &mut FileDescriptor, _command: u8) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
    fn seek(&self, _fd: &mut FileDescriptor, _offset: u32) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
}
