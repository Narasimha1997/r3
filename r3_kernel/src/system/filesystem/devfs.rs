extern crate alloc;
extern crate log;
extern crate spin;

use crate::system::filesystem::vfs::{FILESYSTEM, VFS};
use crate::system::filesystem::{FDOps, FSError, FSOps, FileDescriptor, MountInfo, SeekType};
use alloc::{boxed::Box, string::String, vec::Vec};

use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};

pub trait DevOps {
    fn read(&self, fd: &mut DevFSDescriptor, buffer: &mut [u8]) -> Result<usize, FSError>;
    fn write(&self, fd: &mut DevFSDescriptor, buffer: &[u8]) -> Result<usize, FSError>;
    fn ioctl(&self, command: usize, arg: usize) -> Result<usize, FSError>;
    fn seek(&self, fd: &mut DevFSDescriptor, offset: u32, st: SeekType) -> Result<u32, FSError>;
}

pub struct DevFSEntry {
    pub name: String,
    pub major: u32,
    pub minor: u32,
    pub device: Box<dyn DevOps + Sync + Send>,
    pub ref_count: usize,
}

lazy_static! {
    pub static ref DEV_FS: Mutex<Vec<DevFSEntry>> = Mutex::new(Vec::new());
}

#[derive(Debug, Clone)]
pub struct DevFSDescriptor {
    pub flags: u32,
    pub major: u32,
    pub minor: u32,
    /// some devices that require offset based reads/writes can use this.
    pub offset: u32,
}

#[derive(Debug, Clone)]
/// a driver that handles all these operations:
pub struct DevFSDriver;

impl DevFSDriver {
    pub fn new() -> Self {
        DevFSDriver {}
    }
}

#[inline]
fn get_dev_index(
    locked_dev: &MutexGuard<Vec<DevFSEntry>>,
    major: u32,
    minor: u32,
) -> Option<usize> {
    for (index, device) in locked_dev.iter().enumerate() {
        if major == device.major && minor == device.minor {
            return Some(index);
        }
    }
    None
}

impl FSOps for DevFSDriver {
    fn open(&mut self, path: &str, flags: u32) -> Result<FileDescriptor, FSError> {
        // look for the device by it's path and return the file-descriptor:
        let mut devfs_lock = DEV_FS.lock();
        for entry in devfs_lock.iter_mut() {
            if entry.name == path {
                entry.ref_count += 1;
                // prepare devfs handle:
                return Ok(FileDescriptor::DevFSNode(DevFSDescriptor {
                    flags,
                    major: entry.major,
                    minor: entry.minor,
                    offset: 0,
                }));
            }
        }

        Err(FSError::NotFound)
    }

    fn close(&self, fd: &FileDescriptor) -> Result<(), FSError> {
        match fd {
            FileDescriptor::DevFSNode(devfd) => {
                let mut devfs_lock = DEV_FS.lock();
                if let Some(dev_index) = get_dev_index(&devfs_lock, devfd.major, devfd.minor) {
                    let entry = devfs_lock.get_mut(dev_index).unwrap();
                    if entry.major == devfd.major && entry.minor == devfd.minor {
                        // close this device:
                        if entry.ref_count > 0 {
                            entry.ref_count -= 1;
                        }
                        return Ok(());
                    }
                }
            }
            _ => {}
        }
        Err(FSError::NotFound)
    }
}

impl FDOps for DevFSDriver {
    fn read(&self, fd: &mut FileDescriptor, buffer: &mut [u8]) -> Result<usize, FSError> {
        match fd {
            FileDescriptor::DevFSNode(devfd) => {
                let mut dev_lock = DEV_FS.lock();
                if let Some(dev_index) = get_dev_index(&dev_lock, devfd.major, devfd.minor) {
                    let entry: &mut DevFSEntry = dev_lock.get_mut(dev_index).unwrap();
                    // perform read operation on the device
                    return entry.device.as_ref().read(devfd, buffer);
                }
            }
            _ => {}
        }

        Err(FSError::NotFound)
    }

    fn write(&self, fd: &mut FileDescriptor, buffer: &[u8]) -> Result<usize, FSError> {
        match fd {
            FileDescriptor::DevFSNode(devfd) => {
                let mut dev_lock = DEV_FS.lock();
                if let Some(dev_index) = get_dev_index(&dev_lock, devfd.major, devfd.minor) {
                    let entry: &mut DevFSEntry = dev_lock.get_mut(dev_index).unwrap();
                    // perform write operation on the device
                    return entry.device.as_ref().write(devfd, &buffer);
                }
            }
            _ => {}
        }

        Err(FSError::NotFound)
    }

    fn ioctl(&self, fd: &mut FileDescriptor, command: usize, arg: usize) -> Result<usize, FSError> {
        match fd {
            FileDescriptor::DevFSNode(devfd) => {
                let mut dev_lock = DEV_FS.lock();
                if let Some(dev_index) = get_dev_index(&dev_lock, devfd.major, devfd.minor) {
                    let entry: &mut DevFSEntry = dev_lock.get_mut(dev_index).unwrap();
                    // perform ioctl operation on the device
                    return entry.device.as_ref().ioctl(command, arg);
                }
            }
            _ => {}
        }

        Err(FSError::NotFound)
    }

    fn seek(&self, fd: &mut FileDescriptor, offset: u32, st: SeekType) -> Result<u32, FSError> {
        match fd {
            FileDescriptor::DevFSNode(devfd) => {
                let mut dev_lock = DEV_FS.lock();
                if let Some(dev_index) = get_dev_index(&dev_lock, devfd.major, devfd.minor) {
                    let entry: &mut DevFSEntry = dev_lock.get_mut(dev_index).unwrap();
                    // perform read operation on the device
                    return entry.device.as_ref().seek(devfd, offset, st);
                }
            }
            _ => {}
        }

        Err(FSError::NotFound)
    }
}

/// mounts the devfs on the given path:
pub fn mount_devfs(path: &str) {
    let mount_info = MountInfo::DevFS(DevFSDriver::new());
    let mut fs_lock: MutexGuard<VFS> = FILESYSTEM.lock();
    fs_lock
        .mount_at(path, mount_info)
        .expect("Error when mounting devfs");
    log::info!("Mounted devfs at {}", path);
}

/// register a new devfs device
pub fn register_device(
    name: &str,
    major: u32,
    minor: u32,
    device: Box<dyn DevOps + Sync + Send>,
) -> Result<(), FSError> {
    // exists?
    let mut devfs_lock = DEV_FS.lock();
    if let Some(_) = get_dev_index(&devfs_lock, major, minor) {
        return Err(FSError::AlreadyExist);
    }

    let device_entry = DevFSEntry {
        name: String::from(name),
        major,
        minor,
        device,
        ref_count: 0,
    };

    devfs_lock.push(device_entry);
    Ok(())
}

/// unregister device
pub fn unregister_device(major: u32, minor: u32) -> Result<(), FSError> {
    let mut devfs_lock = DEV_FS.lock();
    if let Some(index) = get_dev_index(&devfs_lock, major, minor) {
        devfs_lock.remove(index);
        return Ok(());
    }

    Err(FSError::NotFound)
}
