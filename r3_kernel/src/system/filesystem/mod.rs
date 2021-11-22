extern crate alloc;

pub mod paths;
pub mod vfs;
pub mod devfs;

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
}

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub path: String,
    pub node: NodeType,
}


pub trait FSOps {
    fn open(&self, _path: &str) -> Result<FileDescriptor, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn close(&self) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn read(&self, _path: &str, _buffer: &[u8]) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn write(&self, _path: &str, _buffer: &[u8]) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }

    // TODO: Implement more
}