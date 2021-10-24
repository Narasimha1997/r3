extern crate bootloader;
extern crate spin;

use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct BootStruct<'a> {
    pub boot_info: Option<&'a BootInfo>,
}

impl<'a> BootStruct<'a> {
    pub fn empty() -> Self {
        BootStruct { boot_info: None }
    }

    pub fn save(&mut self, boot_info: &'a BootInfo) {
        self.boot_info = Some(boot_info);
    }
}

lazy_static! {
    pub static ref BOOT_INFO: Mutex<BootStruct<'static>> = Mutex::new(BootStruct::empty());
}

pub fn save_bootinfo(info: &'static BootInfo) {
    BOOT_INFO.lock().save(info);
}

// BootProtocl is an abstract structure that encapsulates all the boot level information.
// this abstraction helps us to port a multiboot2 based bootloader in the future by only changing
// this implementation than the whole codebase.
pub struct BootProtocol {}

impl BootProtocol {
    pub fn create(info: &'static BootInfo) {
        BOOT_INFO.lock().save(info);
    }
}
