extern crate alloc;

pub mod devfs;
pub mod paths;
pub mod vfs;
pub mod ustar;

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
