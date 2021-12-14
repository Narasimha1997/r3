extern crate alloc;
extern crate log;
extern crate spin;

use crate::cpu;
use crate::drivers::display::fb_text::{FramebufferLines, FramebufferText};
use crate::drivers::display::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::drivers::display::framebuffer::{Framebuffer, Pixel};
use crate::drivers::keyboard::PC_KEYBOARD;

use crate::system::filesystem::devfs::{DevFSDescriptor, DevOps};
use crate::system::filesystem::{FSError, SeekType};

use alloc::{format, string::String, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

const END_OF_TEXT: char = '\x03';
const END_OF_TRANSFER: char = '\x04';
const BACKSPACE: char = '\x08';
const ESCAPE: char = '\x1B';

lazy_static! {
    pub static ref STDIN_QUEUE: Mutex<InputQueue> = Mutex::new(InputQueue::empty());
    pub static ref SYSTEM_TTY: Mutex<BlockingSystemTerminal> =
        Mutex::new(BlockingSystemTerminal::new());
}

pub struct InputQueue {
    keybuf: Vec<u8>,
}

impl InputQueue {
    pub fn empty() -> Self {
        InputQueue { keybuf: Vec::new() }
    }

    #[inline]
    pub fn push(&mut self, key: char) {
        self.keybuf.push(key as u8);
    }

    #[inline]
    pub fn pop_first(&mut self) -> Option<char> {
        if self.keybuf.is_empty() {
            return None;
        } else {
            return Some(self.keybuf.remove(0) as char);
        }
    }

    #[inline]
    pub fn pop_last(&mut self) -> Option<char> {
        self.keybuf.pop().map(|byte| byte as char)
    }

    #[inline]
    pub fn drain(&mut self) {
        self.keybuf.clear();
    }

    #[inline]
    pub fn has_end(&self, ch: char) -> bool {
        if let Some(last) = self.keybuf.last() {
            if *last == ch as u8 {
                return true;
            }
        }

        return false;
    }
}

#[derive(Debug, Clone)]
/// This terminal driver works in blocking mode
/// The read/write requests are blocked until fullfilled.
pub struct BlockingSystemTerminal {
    pub lines: FramebufferLines,
    pub color: Pixel,
    pub echo_input: bool,
    pub parse: bool,
    pub max_rows: usize,
    pub max_cols: usize,
}

impl BlockingSystemTerminal {
    pub fn clear(&mut self) {
        let mut fb = Framebuffer::get_buffer_lock().as_ref().unwrap().lock();
        // clear off the framebuffer
        Framebuffer::fill(
            &mut fb,
            Pixel {
                r: 0,
                g: 0,
                b: 0,
                channel: 0,
            },
        );

        self.lines.row_line = 0;
        self.lines.col_line = 0;
    }

    pub fn new() -> Self {
        let fb = Framebuffer::get_buffer_lock().as_ref().unwrap().lock();
        // clear off the framebuffer
        BlockingSystemTerminal {
            lines: FramebufferLines {
                row_line: 0,
                col_line: 0,
            },
            color: Pixel {
                b: 0,
                g: 255,
                r: 0,
                channel: 0,
            },
            echo_input: true,
            parse: true,
            max_cols: fb.width / FONT_WIDTH,
            max_rows: fb.height / FONT_HEIGHT,
        }
    }

    #[inline]
    pub fn disable_parsing(&mut self) {
        self.parse = false;
    }

    #[inline]
    pub fn enable_parsing(&mut self) {
        self.parse = true;
    }

    #[inline]
    pub fn disable_echo(&mut self) {
        self.echo_input = false;
    }

    #[inline]
    pub fn enable_echo(&mut self) {
        self.echo_input = true;
    }

    #[inline]
    pub fn process_key(&mut self, key: char) {
        let mut input_queue = STDIN_QUEUE.lock();

        // match special tokens
        // 1. backspace
        if key == BACKSPACE && self.parse {
            // this is a backspace
            if let Some(last_char) = input_queue.pop_last() {
                // how many times do we pop?
                if self.echo_input {
                    let n_times = match last_char {
                        END_OF_TEXT | END_OF_TRANSFER | ESCAPE => 2,
                        _ => 1,
                    };
                    let mut fb = Framebuffer::get_buffer_lock().as_ref().unwrap().lock();
                    for _ in 0..n_times {
                        FramebufferText::print_backspace(
                            &mut fb,
                            &mut self.lines,
                            self.max_cols,
                            self.color,
                        );
                    }

                    FramebufferText::print_string(&mut fb, &format!("_"), self.color, &self.lines);
                }
            }
        } else {
            // write the char to the buffer:
            input_queue.push(key);
            if self.echo_input {
                let mut fb = Framebuffer::get_buffer_lock().as_ref().unwrap().lock();
                let to_write = match key {
                    END_OF_TEXT => format!("^C"),
                    END_OF_TRANSFER => format!("^D"),
                    ESCAPE => format!("^["),
                    _ => format!("{}", key),
                };

                let new_lines =
                    FramebufferText::print_string(&mut fb, &to_write, self.color, &self.lines);

                self.lines = new_lines;

                FramebufferText::print_string(&mut fb, &format!("_"), self.color, &self.lines);
            }
        }
    }

    #[inline]
    pub fn write(&mut self, buffer: &[u8]) {
        let string = String::from_utf8_lossy(buffer);

        // write this string:
        let mut fb_lock = Framebuffer::get_buffer_lock().as_ref().unwrap().lock();
        let new_lines =
            FramebufferText::print_string(&mut fb_lock, &string, self.color, &self.lines);
        self.lines = new_lines;
    }

    #[inline]
    pub fn to_offset(&self) -> usize {
        self.lines.row_line * self.max_cols + self.lines.col_line
    }

    #[inline]
    pub fn to_lines(&mut self, offset: usize) -> Result<(), FSError> {
        let n_rows = offset / self.max_cols;
        let n_cols = offset - n_rows * self.max_cols;

        if n_rows > self.max_rows {
            return Err(FSError::InvalidSeek);
        }

        self.lines.col_line = n_cols;
        self.lines.row_line = n_rows;
        Ok(())
    }

    #[inline]
    pub fn end(&mut self) -> usize {
        self.lines.col_line = self.max_cols;
        self.lines.row_line = self.max_rows;
        self.max_rows * self.max_cols
    }
}

#[inline]
pub fn polling_pop() -> char {
    cpu::enable_interrupts();

    loop {
        cpu::halt();
        cpu::disable_interrupts();

        let read_char = STDIN_QUEUE.lock().pop_first();

        cpu::enable_interrupts();

        if read_char.is_some() {
            return read_char.unwrap();
        }
    }
}

pub fn polling_read_till(till: char, buffer: &mut [u8]) -> usize {
    cpu::enable_interrupts();

    loop {
        cpu::halt();
        cpu::disable_interrupts();

        let mut stdin = STDIN_QUEUE.lock();
        let read_size = if !stdin.keybuf.is_empty() && stdin.has_end(till) {
            stdin.keybuf.truncate(buffer.len() - 1);
            let keybuf_length = stdin.keybuf.len();
            buffer[0..keybuf_length].copy_from_slice(&stdin.keybuf);
            stdin.drain();
            keybuf_length
        } else {
            0
        };

        cpu::enable_interrupts();
        if read_size != 0 {
            return read_size;
        }
    }
}

pub fn on_kbd_data(c: char) {
    SYSTEM_TTY.lock().process_key(c);
}

pub fn register_consumer() {
    PC_KEYBOARD.lock().set_handler(on_kbd_data)
}

pub struct TTYDriver;

impl TTYDriver {
    pub fn empty() -> Self {
        TTYDriver {}
    }
}

impl DevOps for TTYDriver {
    fn write(&self, fd: &mut DevFSDescriptor, buffer: &[u8]) -> Result<usize, FSError> {
        let mut tty_lock = SYSTEM_TTY.lock();
        // update the fd to current row, col
        fd.offset = tty_lock.to_offset() as u32;
        // write to the framebuffer:
        tty_lock.write(&buffer);
        // update the file-descriptor
        fd.offset = tty_lock.to_offset() as u32;
        Ok(buffer.len())
    }

    fn read(&self, _fd: &mut DevFSDescriptor, buffer: &mut [u8]) -> Result<usize, FSError> {
        if buffer.len() <= 4 {
            // read a single character
            let ch = polling_pop();
            buffer[0] = ch as u8;
            return Ok(1);
        } else {
            // read until the end
            let read_until = polling_read_till('\n', buffer);
            Ok(read_until)
        }
    }

    fn seek(&self, fd: &mut DevFSDescriptor, offset: u32, st: SeekType) -> Result<u32, FSError> {
        match st {
            SeekType::SEEK_END => {
                let end = SYSTEM_TTY.lock().end();
                fd.offset = end as u32;
                return Ok(fd.offset);
            }
            SeekType::SEEK_SET => {
                let set_res = SYSTEM_TTY.lock().to_lines(offset as usize);
                if set_res.is_err() {
                    return Err(set_res.unwrap_err());
                }
                // set the new offset
                fd.offset = offset;
            }
            SeekType::SEEK_CUR => {
                let set_res = SYSTEM_TTY.lock().to_lines((fd.offset + offset) as usize);
                if set_res.is_err() {
                    return Err(set_res.unwrap_err());
                }

                // set the new offset
                fd.offset = offset + fd.offset;
            }
        }

        Ok(fd.offset)
    }

    fn ioctl(&self, _command: usize, _arg: usize) -> Result<usize, FSError> {
        Ok(0)
    }
}

// TODO: Find a best wat to mitigate this
unsafe impl Send for TTYDriver {}
unsafe impl Sync for TTYDriver {}

pub fn initialize() {
    // touch the tty device:
    SYSTEM_TTY.lock().clear();
    STDIN_QUEUE.lock().drain();
    // set this as default keyboard consumer:
    register_consumer();

    log::debug!("Initialized system terminal.");
}
