extern crate log;

use crate::boot_proto::BootProtocol;

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

/// Represents a framebuffer and other metadata used to control
/// different functions of framebuffer.
pub struct Framebuffer {
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

impl Framebuffer {
    pub fn new(boot_info: BootProtocol) -> Self {
        let fb_slice_opt = BootProtocol::get_framebuffer_slice();
        if fb_slice_opt.is_none() {
            panic!("Framebuffer address not provided by bootloader.");
        }

        let fb_info_opt = BootProtocol::get_framebuffer_info();
        if fb_info_opt.is_none() {
            panic!("Framebuffer information is not provided by bootloader.");
        }

        let fb_info = fb_info_opt.unwrap();

        Framebuffer {
            buffer: fb_slice_opt.unwrap(),
            width: fb_info.horizontal_resolution,
            height: fb_info.vertical_resolution,
            bytes_per_pixel: fb_info.bytes_per_pixel,
        }
    }

    #[inline]
    fn index_in_bounds(&self, index: &FramebufferIndex) -> bool {
        index.x < self.width && index.y < self.height
    }

    #[inline]
    fn index_to_offset(&self, index: FramebufferIndex) -> Option<usize> {
        if self.index_in_bounds(&index) {
            Some((index.y * self.width + index.x) * self.bytes_per_pixel)
        } else {
            None
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, pixel: Pixel, index: FramebufferIndex) {
        if let Some(offset) = self.index_to_offset(index) {
            self.buffer[offset] = pixel.b;
            self.buffer[offset + 1] = pixel.g;
            self.buffer[offset + 2] = pixel.r;
            self.buffer[offset + 3] = pixel.channel;
        }
    }

    #[inline]
    pub fn get_pixel(&self, index: FramebufferIndex) -> Option<Pixel> {
        if let Some(offset) = self.index_to_offset(index) {
            return Some(Pixel {
                b: self.buffer[offset],
                g: self.buffer[offset + 1],
                r: self.buffer[offset + 2],
                channel: self.buffer[offset + 4],
            });
        }

        None
    }
}
