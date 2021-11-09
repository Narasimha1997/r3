extern crate log;

use crate::boot_proto::BootProtocol;
use crate::mm::{paging::KernelVirtualMemoryManager, VirtualAddress};

pub fn dump_phy_address() {
    if let Some(fb_slice) = BootProtocol::get_framebuffer_slice() {
        let addr =
            KernelVirtualMemoryManager::pt().translate(VirtualAddress::from_ptr(&fb_slice[0]));
        log::info!(
            "Framebuffer at: virt_address={:p}, phy_addr={}.",
            &fb_slice[0],
            addr.unwrap().as_u64()
        );
    }
}

pub struct Pixel {
    /// represents blueness
    pub b: u8,
    /// represents greeness
    pub g: u8,
    /// represents redness
    pub r: u8,
    /// this byte is 0 in BGR mode, in BGRA, it is alphaness.
    pub channel: u8,
}

/// Represents a framebuffer and other metadata used to control
/// different functions of framebuffer.
pub struct Framebuffer {
    /// contains a reference to framebuffer slice
    pub buffer: &'static [u8],
    pub width: usize,
    pub height: usize,
}
