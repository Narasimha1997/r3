extern crate alloc;
extern crate spin;

use crate::system::filesystem::{FSError, MountInfo};

use alloc::{boxed::Box, string::String, vec::Vec};
use spin::Mutex;

use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct VFSMountPoint {
    pub path: String,
    pub mountinfo: Box<MountInfo>,
    pub ref_count: usize,
}

impl VFSMountPoint {
    #[inline]
    pub fn incr_refcount(&mut self) {
        self.ref_count +=1;
    }

    #[inline]
    pub fn decr_refcount(&mut self) {
        if self.ref_count > 0 {
            self.ref_count -=1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct VFS {
    pub mountpoints: Vec<VFSMountPoint>,
}

impl VFS {
    pub fn empty() -> Self {
        VFS {
            mountpoints: Vec::new(),
        }
    }

    pub fn mount_at(&mut self, path: &str, mountinfo: MountInfo) -> Result<(), FSError> {
        // exists?
        if let Some(_) = self.get_mount_index(&path) {
            return Err(FSError::AlreadyExist);
        }

        // create a mount:
        let mountpoint = VFSMountPoint {
            path: String::from(path),
            mountinfo: Box::new(mountinfo),
            ref_count: 0
        };

        self.mountpoints.push(mountpoint);
        return Ok(());
    }

    /// is the mount point exists at given path? if yes return the index
    /// or return `None`.
    pub fn get_mount_index(&self, path: &str) -> Option<usize> {
        for (idx, mount) in self.mountpoints.iter().enumerate() {
            if mount.path == path {
                return Some(idx);
            }
        }
        None
    }

    pub fn remove_mount(&mut self, path: &str) -> Result<(), FSError> {
        if let Some(mount_index) = self.get_mount_index(path) {
            if (&self.mountpoints[mount_index]).ref_count != 0 {
                // device is being referenced by other mountpoints:
                return Err(FSError::Busy);
            }

            self.mountpoints.remove(mount_index);
            return Ok(());
        }
        Err(FSError::NotFound)
    }

    /// returns the index of matching mountpoint
    /// resolves the path using longest prefix search to get the
    /// latest mountpoint matching the given path prefix
    /// the path provided to this function must be cananoized path.
    pub fn get_matching_mountpoint(&self, path: &str) -> Result<(usize, usize), FSError> {
        let mut curr_prefix_length = 0;
        let mut curr_index: i32 = -1;

        for (idx, mountpoint) in self.mountpoints.iter().enumerate() {
            if path.starts_with(&mountpoint.path) {
                // longest prefix?
                let new_length = mountpoint.path.len();
                if mountpoint.path.len() > curr_prefix_length {
                    curr_prefix_length = new_length;
                    curr_index = idx as i32;
                }
            }
        }

        if curr_index < 0 {
            // no mountpoint found:
            return Err(FSError::NotFound);
        }

        // safe to convert because index >= 0
        Ok((curr_index as usize, curr_prefix_length))
    }

    /// dumps all the mountpoints
    pub fn debug_dump_mountpoints(&self) {
        for mp in &self.mountpoints {
            log::debug!("mountpath={}, mount_type={:?}", mp.path, mp.mountinfo)
        }
    }
}

lazy_static! {
    pub static ref FILESYSTEM: Mutex<VFS> = Mutex::new(VFS::empty());
}

pub fn setup_fs() {
    log::info!(
        "VFS set-up successful, n_mountpoints={}",
        FILESYSTEM.lock().mountpoints.len()
    )
}
