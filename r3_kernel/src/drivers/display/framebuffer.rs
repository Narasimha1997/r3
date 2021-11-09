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

        // write some bytes
        for i in 0..10000 {
            fb_slice[i] = 0;
        }
    }
}
