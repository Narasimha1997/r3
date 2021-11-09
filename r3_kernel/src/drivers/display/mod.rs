pub mod framebuffer;
pub mod font;
pub mod fb_text;

use framebuffer::{Pixel, setup_framebuffer, Framebuffer};

pub fn init() {
    setup_framebuffer();

    let black = Pixel{b: 255, g: 255, r: 255, channel: 0};
    Framebuffer::fill(black);
}