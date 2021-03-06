extern crate log;
extern crate spin;

use crate::boot_proto::BootProtocol;
use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};

#[derive(Debug, Clone, Copy)]
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

/// Represents a framebuffer memory region and other metadata used to control
/// different functions of framebuffer.
pub struct FramebufferMemory {
    /// contains a reference to framebuffer slice
    pub buffer: &'static mut [u8],
    pub width: usize,
    pub height: usize,
    pub bytes_per_pixel: usize,
}

pub struct FramebufferIndex {
    pub x: usize,
    pub y: usize,
}

impl FramebufferMemory {
    /// creates a new frame buffer memory region over the framebuffer area
    /// provided by the bootloader.
    pub fn new() -> Option<Self> {
        let fb_slice_opt = BootProtocol::get_framebuffer_slice();
        if fb_slice_opt.is_none() {
            log::error!(
                "Could not initialize framebuffer, because
                 bootloader did not provide framebuffer address."
            );
            return None;
        }

        let fb_info_opt = BootProtocol::get_framebuffer_info();
        if fb_info_opt.is_none() {
            log::error!(
                "Could not initialize framebuffer, 
                 because the bootloader did not provide framebuffer info."
            );
            return None;
        }

        let fb_info = fb_info_opt.unwrap();

        Some(FramebufferMemory {
            buffer: fb_slice_opt.unwrap(),
            width: fb_info.horizontal_resolution,
            height: fb_info.vertical_resolution,
            bytes_per_pixel: fb_info.bytes_per_pixel,
        })
    }

    pub fn get_slice_bounded(start: usize, end: usize) -> Option<&'static mut [u8]> {
        let fb_slice_opt = BootProtocol::get_framebuffer_slice();
        if fb_slice_opt.is_none() {
            return None;
        }

        Some(&mut fb_slice_opt.unwrap()[start..end])
    }

    pub fn get_slice_from(start: usize) -> Option<&'static mut [u8]> {
        let fb_slice_opt = BootProtocol::get_framebuffer_slice();
        if fb_slice_opt.is_none() {
            return None;
        }

        Some(&mut fb_slice_opt.unwrap()[start..])
    }
}

/// LockedFramebuffer represents a framebuffer memory region with mutex.
pub type LockedFramebuffer = Mutex<FramebufferMemory>;

/// initializes the framebuffer
fn init_framebuffer() -> Option<Mutex<FramebufferMemory>> {
    let fb_opt = FramebufferMemory::new();
    if fb_opt.is_none() {
        return None;
    }

    Some(Mutex::new(fb_opt.unwrap()))
}

lazy_static! {
    pub static ref FRAMEBUFFER: Option<LockedFramebuffer> = init_framebuffer();
}

/// Set of control functions used for writing pixels to frame buffer
pub struct Framebuffer;

impl Framebuffer {
    #[inline]
    pub fn get_buffer_lock() -> &'static Option<LockedFramebuffer> {
        &FRAMEBUFFER
    }

    #[inline]
    pub fn index_in_bounds(fb: &FramebufferMemory, index: &FramebufferIndex) -> bool {
        index.x < fb.width && index.y < fb.height
    }

    #[inline]
    fn index_to_offset(fb: &FramebufferMemory, index: FramebufferIndex) -> Option<usize> {
        if Framebuffer::index_in_bounds(&fb, &index) {
            Some((index.y * fb.width + index.x) * fb.bytes_per_pixel)
        } else {
            None
        }
    }

    #[inline]
    pub fn set_pixel(fb: &mut FramebufferMemory, pixel: Pixel, index: FramebufferIndex) {
        if let Some(offset) = Framebuffer::index_to_offset(&fb, index) {
            fb.buffer[offset] = pixel.b;
            fb.buffer[offset + 1] = pixel.g;
            fb.buffer[offset + 2] = pixel.r;
            fb.buffer[offset + 3] = pixel.channel;
        }
    }

    #[inline]
    pub fn get_pixel(fb: &FramebufferMemory, index: FramebufferIndex) -> Option<Pixel> {
        if let Some(offset) = Framebuffer::index_to_offset(fb, index) {
            return Some(Pixel {
                b: fb.buffer[offset],
                g: fb.buffer[offset + 1],
                r: fb.buffer[offset + 2],
                channel: fb.buffer[offset + 4],
            });
        }

        None
    }

    pub fn fill(fb: &mut MutexGuard<FramebufferMemory>, pixel: Pixel) {
        let bps = fb.bytes_per_pixel;
        Framebuffer::fill_region(fb.buffer, pixel, bps);
    }

    pub fn fill_region(fb_region_slice: &mut [u8], pixel: Pixel, bps: usize) {
        if fb_region_slice.len() % bps != 0 {
            return;
        }

        let mut offset = 0;
        while offset < fb_region_slice.len() {
            fb_region_slice[offset] = pixel.b;
            fb_region_slice[offset + 1] = pixel.g;
            fb_region_slice[offset + 2] = pixel.r;
            fb_region_slice[offset + 3] = pixel.channel;
            offset += bps;
        }
    }
}

/// this method initializes the framebuffer, in other words
/// it dereferences the framebuffer memory region which cases
/// the lazy_static struct to initialize.
pub fn setup_framebuffer() {
    if FRAMEBUFFER.is_none() {
        log::error!("Fraebuffer set-up failed, system display will not work.");
    }

    let fb_ref = FRAMEBUFFER.as_ref().unwrap().lock();

    log::info!(
        "Framebuffer initialized, address={:p}, width={}, height={}.",
        &fb_ref.buffer[0],
        fb_ref.width,
        fb_ref.height
    );
}
