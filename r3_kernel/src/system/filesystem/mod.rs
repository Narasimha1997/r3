extern crate alloc;

pub mod devfs;
pub mod paths;
pub mod vfs;

#[derive(Debug, Clone)]
pub enum MountInfo {
    DevFS(devfs::DevFSDriver),
    MemFS,
    BlockFS,
}

#[derive(Debug, Clone)]
/// Represents an entry node in VFS tree
pub enum FileDescriptor {
    DevFSNode(devfs::DevFSDescriptor),
    Ext2Node,
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
    fn read(&self, _fd: &FileDescriptor, _buffer: &[u8]) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
    fn write(&self, _fd: &FileDescriptor, _buffer: &[u8]) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
    fn ioctl(&self, _fd: &FileDescriptor, _command: u8) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
}
