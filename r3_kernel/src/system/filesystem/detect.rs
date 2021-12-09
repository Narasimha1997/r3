extern crate alloc;
extern crate log;
extern crate spin;

use crate::system::filesystem::devfs::{DevFSDescriptor, DEV_FS, DevOps};
use crate::system::filesystem::ustar::mount_tarfs;

use alloc::boxed::Box;

/// Devices capable of storage have major number 2
const STORAGE_DEVICE_MAJOR: usize = 2;

pub fn check_tarfs(major: u32, minor: u32, device: &Box<dyn DevOps + Sync + Send>) -> bool {
    // 1. create a file-descriptor
    let mut fd = DevFSDescriptor {
        flags: 0,
        major,
        minor,
        offset: 0,
    };

    let mut buffer: [u8; 512] = [0; 512];

    // read the first 512 bytes
    let result = device.read(&mut fd, &mut buffer);
    if result.is_err() {
        log::error!("{:?}", result);
        return false;
    }

    if buffer[257..257 + 5] == [b'u', b's', b't', b'a', b'r'] {
        // tarfs is detected
        return true;
    }

    false
}

pub fn detect_filesystems() {
    // dealing with locks!
    for dev_entry in DEV_FS.lock().iter() {
        if dev_entry.major == STORAGE_DEVICE_MAJOR as u32 {
            if check_tarfs(dev_entry.major, dev_entry.minor, &dev_entry.device) {
                // detected a tarfs
                log::info!("Detected TAR filesystem on device /dev/{}", dev_entry.name);
                mount_tarfs(&dev_entry.name, "/sbin");
            } else {
                log::warn!("No usuable filesystem detected on /dev/{}", dev_entry.name);
            }
        }
    }
}
