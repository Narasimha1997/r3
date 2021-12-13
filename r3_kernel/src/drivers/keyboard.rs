extern crate pc_keyboard;
extern crate spin;

use crate::cpu::io::Port;

use lazy_static::lazy_static;
use pc_keyboard::{layouts::Us104Key, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use spin::Mutex;

type OnKeyCallback = fn(char) -> ();

const KEYBOARD_DATA_PORT: usize = 0x60;

/// Wraps up PC keyboard capabilities provided by pc_keyboard crate.
pub struct PCKeyboardController {
    pub layout: Keyboard<Us104Key, ScancodeSet1>,
    pub read_port: Port,
    pub on_key: Option<OnKeyCallback>,
}

impl PCKeyboardController {
    pub fn new() -> Self {
        PCKeyboardController {
            layout: Keyboard::new(Us104Key, ScancodeSet1, HandleControl::MapLettersToUnicode),
            read_port: Port::new(KEYBOARD_DATA_PORT, true),
            on_key: None,
        }
    }

    #[inline]
    fn read_raw(&self) -> u8 {
        self.read_port.read_u8()
    }

    #[inline]
    pub fn set_handler(&mut self, handler: OnKeyCallback) {
        self.on_key = Some(handler);
    }

    #[inline]
    fn send_control(&self, control_char: char) {
        if let Some(handler) = self.on_key {
            handler('\x1B');
            handler('[');
            handler(control_char);
        }
    }

    #[inline]
    pub fn read_key(&mut self) {
        let raw_keybyte = self.read_raw();

        if self.on_key.is_none() {
            return;
        }

        let handler = self.on_key.unwrap();
        if let Ok(partial_keycode_opt) = self.layout.add_byte(raw_keybyte) {
            let partial_keycode = partial_keycode_opt.unwrap();
            if let Some(key_event) = self.layout.process_keyevent(partial_keycode) {
                // is this a raw control key or a utf character key
                if let DecodedKey::Unicode(utf_code) = key_event {
                    handler(utf_code);
                } else {
                    match key_event {
                        DecodedKey::RawKey(KeyCode::ArrowUp) => self.send_control('A'),
                        DecodedKey::RawKey(KeyCode::ArrowDown) => self.send_control('B'),
                        DecodedKey::RawKey(KeyCode::AltLeft) => self.send_control('D'),
                        DecodedKey::RawKey(KeyCode::AltRight) => self.send_control('C'),
                        _ => {}
                    }
                }
            }
        }
    }
}

lazy_static! {
    pub static ref PC_KEYBOARD: Mutex<PCKeyboardController> =
        Mutex::new(PCKeyboardController::new());
}
