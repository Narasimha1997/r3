pub mod fb_text;
pub mod font;
pub mod framebuffer;

use framebuffer::{setup_framebuffer, Framebuffer, Pixel};


pub fn init() {
    setup_framebuffer();

    let fb_locked_opt = Framebuffer::get_buffer_lock();
    if fb_locked_opt.is_some() {
        let black = Pixel {
            b: 0,
            g: 0,
            r: 0,
            channel: 0,
        };

        let mut fb_lock = fb_locked_opt.as_ref().unwrap().lock();
        Framebuffer::fill(&mut fb_lock, black);
    }
}
