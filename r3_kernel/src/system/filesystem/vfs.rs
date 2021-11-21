extern crate alloc;
extern crate spin;

use crate::system::filesystem::paths;
use crate::system::filesystem::{FSError, NodeType};

use lazy_static::lazy_static;

use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};
use spin::Mutex;

#[derive(Debug, Clone)]
pub struct VFSEntry {
    // a dummy variable used for quick lookups
    pub name: String,
    pub node: Option<Box<NodeType>>,
    pub children: Vec<VFSEntry>,
}

impl VFSEntry {
    pub fn get_child(&self, name: &str) -> Option<&VFSEntry> {
        for child in &self.children {
            if child.name == name {
                return Some(child);
            }
        }

        None
    }

    pub fn get_mut_child(&mut self, name: &str) -> Option<&mut VFSEntry> {
        for child in self.children.iter_mut() {
            if child.name == name {
                return Some(child);
            }
        }

        None
    }

    #[inline]
    fn get_child_index(&self, name: &str) -> Option<usize> {
        for idx in 0..self.children.len() {
            if (&self.children[idx]).name == name {
                return Some(idx);
            }
        }

        None
    }

    pub fn remove_child(&mut self, name: &str) -> Result<(), FSError> {
        if let Some(child_idx) = self.get_child_index(&name) {
            self.children.remove(child_idx);
            return Ok(());
        }

        Err(FSError::NotFound)
    }

    pub fn create_child(&mut self, name: &str, node: NodeType) -> Result<(), FSError> {
        if let Some(_) = self.get_child_index(&name) {
            return Err(FSError::AlreadyExist);
        }

        let node_entry = VFSEntry {
            name: String::from(name),
            node: Some(Box::new(node)),
            children: Vec::new(),
        };

        self.children.push(node_entry);
        Ok(())
    }

    pub fn empty() -> VFSEntry {
        VFSEntry {
            name: "/".to_string(),
            node: None,
            children: Vec::new(),
        }
    }
}

lazy_static! {
    pub static ref ROOT: Mutex<VFSEntry> = Mutex::new(VFSEntry::empty());
}
