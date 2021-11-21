extern crate alloc;

pub mod paths;
pub mod vfs;

use alloc::{string::String, vec::Vec};
use core::cell::RefCell;

#[derive(Debug, Clone)]
pub enum MountType {
    DevFS,
    MemFS,
    BlockFS,
}

#[derive(Debug, Clone)]
/// Represents an entry node in VFS tree
pub enum NodeType {
    VFSNode(RefCell<vfs::VFSEntry>),
    DevFSNode,
    Ext2Node,
    Mountpoint,
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
}

pub trait FSOps {
    fn readdir(&self, _path: &str) -> Result<Vec<String>, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn mkdir(&self, _path: &str) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn open(&self, _path: &str) -> Result<NodeType, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn close(&self) -> Result<(), FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn create(&self, _path: &str) -> Result<NodeType, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn exists(&self, _path: &str) -> Result<bool, FSError> {
        Err(FSError::NotYetImplemented)
    }

    fn remove(&self, _path: &str, _is_dir: bool) -> Result<bool, FSError> {
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
