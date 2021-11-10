use crate::drivers::display::font::{get_bit_for_char, FONT_HEIGHT, FONT_WIDTH, LINUX_BOOT_FONT};
use crate::drivers::display::framebuffer;

pub struct FramebufferLines {
    pub row_line: usize,
    pub col_line: usize,
}

pub struct FramebufferText;

impl FramebufferText {
    fn print_ascii_char(
        fb: &mut framebuffer::FramebufferMemory,
        ch: u8,
        color: framebuffer::Pixel,
        r_line: &usize,
        c_line: &usize,
        buffer_width: usize,
        buffer_height: usize,
    ) {
        let start_y = r_line * FONT_HEIGHT;
        let start_x = c_line * FONT_WIDTH;

        let mut j = start_x;
        let mut i = start_y;

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
                    j = start_x;
                }
            }
        }
    }

    pub fn print_string(
        fb: &mut framebuffer::FramebufferMemory,
        string: &str,
        color: framebuffer::Pixel,
        pos: FramebufferLines,
    ) -> FramebufferLines {
        let n_rows = fb.height / FONT_HEIGHT;
        let n_cols = fb.height / FONT_WIDTH;

        let mut c_row = pos.row_line;
        let mut c_col = pos.col_line;

        for ch in string.as_bytes() {
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
                    break;
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
