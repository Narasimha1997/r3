extern crate log;
extern crate spin;

use crate::boot_proto::BootProtocol;
use lazy_static::lazy_static;
use spin::Mutex;

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

    pub fn fill(pixel: Pixel) {
        let fb_opt = Framebuffer::get_buffer_lock();
        if fb_opt.is_none() {
            return;
        }

        let mut fb_lock = fb_opt.as_ref().unwrap().lock();

        let n_bytes = fb_lock.width * fb_lock.height * fb_lock.bytes_per_pixel;
        let mut offset = 0;

        while offset < n_bytes {
            fb_lock.buffer[offset] = pixel.b;
            fb_lock.buffer[offset + 1] = pixel.g;
            fb_lock.buffer[offset + 2] = pixel.r;
            fb_lock.buffer[offset + 3] = pixel.channel;
            offset += fb_lock.bytes_per_pixel;
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
