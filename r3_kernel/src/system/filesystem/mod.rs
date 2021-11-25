extern crate alloc;

pub mod devfs;
pub mod paths;
pub mod vfs;

use alloc::string::String;

#[derive(Debug, Clone)]
pub enum MountInfo {
    DevFS,
    MemFS,
    BlockFS,
}

#[derive(Debug, Clone)]
/// Represents an entry node in VFS tree
pub enum NodeType {
    DevFSNode,
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

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub path: String,
    pub node: NodeType,
}

/// Represents the operations performed on File-System
pub trait FSOps {
    fn open(&self, _path: &str) -> Result<FileDescriptor, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn close(&self, _fd: FileDescriptor) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }
}
