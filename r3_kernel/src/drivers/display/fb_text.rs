extern crate log;
extern crate spin;

use crate::drivers::display::font::{get_bit_for_char, FONT_HEIGHT, FONT_WIDTH, LINUX_BOOT_FONT};
use crate::drivers::display::framebuffer;

use core::fmt;
use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};

pub struct FramebufferLines {
    pub row_line: usize,
    pub col_line: usize,
}

pub struct FramebufferText;

impl FramebufferText {
    pub fn scroll(fb: &mut MutexGuard<framebuffer::FramebufferMemory>, _n_lines: usize) {
        // as of now, clear the screen on scroll:
        framebuffer::Framebuffer::fill(
            fb,
            framebuffer::Pixel {
                b: 0,
                g: 0,
                r: 0,
                channel: 0,
            },
        );
    }

    pub fn print_ascii_char(
        fb: &mut MutexGuard<framebuffer::FramebufferMemory>,
        ch: u8,
        color: framebuffer::Pixel,
        r_line: &usize,
        c_line: &usize,
        buffer_width: usize,
        buffer_height: usize,
    ) {
        let start_y = r_line * FONT_HEIGHT;
        let start_x = c_line * FONT_WIDTH;

        let mut j = 0;
        let mut i = 0;

        loop {
            let index = framebuffer::FramebufferIndex {
                y: start_y + i,
                x: start_x + j,
            };

            if framebuffer::Framebuffer::index_in_bounds(&fb, &index) {
                if j >= 1 {
                    let idx = j - 1;
                    let char_font = LINUX_BOOT_FONT[ch as usize][i];
                    if get_bit_for_char(char_font, idx) != 0 {
                        // draw the pixel on framebuffer:
                        framebuffer::Framebuffer::set_pixel(fb, color, index);
                    }
                }

                j = j + 1;
                if j == FONT_WIDTH || start_x + j == buffer_width {
                    i = i + 1;
                    if i == FONT_HEIGHT || start_y + i == buffer_height {
                        return;
                    }
                    j = 0;
                }
            }
        }
    }

    pub fn print_string(
        fb: &mut MutexGuard<framebuffer::FramebufferMemory>,
        string: &str,
        color: framebuffer::Pixel,
        pos: &FramebufferLines,
    ) -> FramebufferLines {
        let n_rows = fb.height / FONT_HEIGHT;
        let n_cols = fb.width / FONT_WIDTH;

        let mut c_row = pos.row_line;
        let mut c_col = pos.col_line;

        for ch in string.as_bytes() {
            if *ch <= 0x20 && *ch >= 0x7e {
                // skip non-printable characters
                continue;
            }

            if *ch == b'\n' {
                c_col = 0;
                c_row += 1;
                continue;
            } else if *ch == b'\t' {
                c_col = c_col + 4;
                continue;
            } else {
                // is this end of the current row?
                if c_col >= n_cols {
                    c_row = c_row + 1;
                    c_col = 0;
                }

                if c_row >= n_rows {
                    // reached end of screen, TODO: Implement scroll
                    // as of now, this function clears the screen entirely.
                    FramebufferText::scroll(fb, 5);
                    c_row = 0;
                    c_col = 0;
                }

                FramebufferText::print_ascii_char(fb, *ch, color, &c_row, &c_col, n_cols, n_rows);
                c_col += 1;
            }
        }

        FramebufferLines {
            row_line: c_row,
            col_line: c_col,
        }
    }
}

pub struct FramebufferLogger {
    pub current_lines: FramebufferLines,
    pub color: framebuffer::Pixel,
}

impl FramebufferLogger {
    pub fn init(color: framebuffer::Pixel) -> Self {
        FramebufferLogger {
            current_lines: FramebufferLines {
                row_line: 0,
                col_line: 0,
            },
            color,
        }
    }

    pub fn write(&mut self, string: &str) {
        let locked_buffer_opt = framebuffer::Framebuffer::get_buffer_lock();
        if locked_buffer_opt.is_none() {
            return;
        }

        let mut locked_buffer = locked_buffer_opt.as_ref().unwrap().lock();

        self.current_lines = FramebufferText::print_string(
            &mut locked_buffer,
            string,
            self.color,
            &self.current_lines,
        );
    }
}

impl fmt::Write for FramebufferLogger {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        self.write(string);
        return Ok(());
    }
}

pub fn setup_framebuffer(color: framebuffer::Pixel) -> Mutex<FramebufferLogger> {
    Mutex::new(FramebufferLogger::init(color))
}

lazy_static! {
    pub static ref FRAMEBUFFER_LOGGER: Mutex<FramebufferLogger> =
        setup_framebuffer(framebuffer::Pixel {
            b: 255,
            g: 255,
            r: 255,
            channel: 0
        });
}
