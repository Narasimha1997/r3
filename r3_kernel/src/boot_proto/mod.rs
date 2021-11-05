extern crate bootloader;
extern crate log;
extern crate spin;

use bootloader::boot_info::{FrameBufferInfo, MemoryRegions};
use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct BootProtoContainer {
    pub boot_info: Option<u64>,
}

// unsafe impl<'a> Sync for BootProtoContainer<'a> {}

impl BootProtoContainer {
    pub fn empty() -> Self {
        BootProtoContainer { boot_info: None }
    }

    pub fn save(&mut self, b_proto: u64) {
        self.boot_info = Some(b_proto);
    }
}

lazy_static! {
    pub static ref BOOT_INFO: Mutex<BootProtoContainer> = Mutex::new(BootProtoContainer::empty());
}

// BootProtocl is an abstract structure that encapsulates all the boot level information.
// this abstraction helps us to port a multiboot2 based bootloader in the future by only changing
// this implementation than the whole codebase.
pub struct BootProtocol {}

impl BootProtocol {
    #[inline]
    fn get_boot_proto() -> Option<&'static BootInfo> {
        if let Some(boot_info_addr) = BOOT_INFO.lock().boot_info {
            return Some(unsafe { (boot_info_addr as *const BootInfo).as_ref().unwrap() });
        }

        None
    }

    pub fn create(info: &'static BootInfo) {
        // translate boot info to boot_proto:
        let boot_struct_addr = (info as *const BootInfo) as u64;
        BOOT_INFO.lock().save(boot_struct_addr);
    }

    pub fn get_memory_regions() -> Option<&'static MemoryRegions> {
        if let Some(bi) = BootProtocol::get_boot_proto() {
            return Some(&bi.memory_regions);
        }

        None
    }

    pub fn get_framebuffer_info() -> Option<FrameBufferInfo> {
        if let Some(bi) = BootProtocol::get_boot_proto() {
            if let Some(fb_struct) = bi.framebuffer.as_ref() {
                return Some(fb_struct.info());
            }

            return None;
        }

        None
    }

    pub fn get_phy_offset() -> Option<u64> {
        if let Some(bi) = BootProtocol::get_boot_proto() {
            return bi.physical_memory_offset.into_option();
        }

        None
    }

    pub fn print_boot_info() {
        if let Some(bi) = BootProtocol::get_boot_proto() {
            // display version:
            log::info!(
                "Bootloader version: {}.{}.{}",
                bi.version_major,
                bi.version_minor,
                bi.version_patch
            );

            log::info!("RSDT Address: {:?}", bi.rsdp_addr);

            log::info!(
                "Memory offset: 0x{:x}",
                bi.physical_memory_offset.into_option().unwrap()
            );

            if let Some(memory_regions) = BootProtocol::get_memory_regions() {
                for region_idx in 0..memory_regions.len() {
                    let region = memory_regions[region_idx];
                    log::info!("{:?}", region);
                }
            } else {
                log::warn!("Boot info doesn't contain memory map information.");
            }

            if let Some(fb_info) = BootProtocol::get_framebuffer_info() {
                log::info!(
                    "Framebuffer info: width={} height={} bps={} pixel_format={:?}",
                    fb_info.horizontal_resolution,
                    fb_info.vertical_resolution,
                    fb_info.bytes_per_pixel,
                    fb_info.pixel_format
                );
            } else {
                log::warn!("Boot info doesn't contain framebuffer information.");
            }
        }
    }
}
