use crate::cpu::io::Port;
use crate::system::filesystem::devfs::{DevOps, DevFSDescriptor};
use crate::system::filesystem::FSError;

use core::fmt;

extern crate spin;

use lazy_static::lazy_static;
use spin::Mutex;

// on x86 architecture, 0x3f8 is the COM port.
const X86_COM_PORT: usize = 0x3f8;
const X86_COM_PORT_EMPTY_FLAG: u8 = 0x20;
const X86_COM_PORT_RECEIVED_FLAG: u8 = 0x1;
const X86_COM_PORT_LOOPBACK_MODE: u8 = 0x1b;

const COM_INVALID_CHAR_BYTE: u8 = b'?';

pub struct UART {
    pub port_0: Port, // read write on this port
    pub port_5: Port, // check transit empty/recv on this port
}

impl UART {
    // set UART in loopback mode and check if same data written is read
    // as it is, if not, the chip is faulty.
    fn chip_works_fine(port_4: &Port, port_0: &Port) -> bool {
        port_4.write_u8(X86_COM_PORT_LOOPBACK_MODE);
        // write some dummy byte
        port_0.write_u8(0xaf);
        // try reading the same byte:
        match port_0.read_u8() {
            0xaf => {
                // set chip in normal mode
                port_4.write_u8(0x0f);
                return true;
            }
            _ => return false,
        }
    }

    pub fn new() -> Option<Self> {
        // configuration
        let port_0 = Port::new(X86_COM_PORT, false);
        let port_1 = Port::new(X86_COM_PORT + 1, false);
        let port_2 = Port::new(X86_COM_PORT + 2, false);
        let port_3 = Port::new(X86_COM_PORT + 3, false);
        let port_4 = Port::new(X86_COM_PORT + 4, false);

        // read only port.
        let port_5 = Port::new(X86_COM_PORT + 5, true);

        port_1.write_u8(0x00);
        port_3.write_u8(0x80);
        port_0.write_u8(0x03);
        port_1.write_u8(0x00);
        port_3.write_u8(0x03);
        port_2.write_u8(0xC7);
        port_4.write_u8(0x0B);

        // check for faulty chip:
        if !UART::chip_works_fine(&port_4, &port_0) {
            return None;
        }

        // return the UART instance:
        Some(UART {
            port_0: port_0,
            port_5: port_5,
        })
    }

    #[inline]
    pub fn transit_empty(&self) -> bool {
        self.port_5.read_u8() & X86_COM_PORT_EMPTY_FLAG != 0
    }

    #[inline]
    pub fn transit_received(&self) -> bool {
        self.port_5.read_u8() & X86_COM_PORT_RECEIVED_FLAG != 0
    }

    pub fn read_u8(&self) -> u8 {
        while !self.transit_received() {}
        self.port_0.read_u8()
    }

    pub fn write_u8(&self, value: u8) {
        while !self.transit_empty() {}
        self.port_0.write_u8(value);
    }

    pub fn write_from_buffer(&self, buffer: &[u8]) {
        for byte in buffer {
            self.write_u8(*byte);
        }
    }

    pub fn read_to_buffer(&self, buffer: &mut [u8]) {
        for index in 0..buffer.len() {
            buffer[index] = self.read_u8();
        }
    }

    #[inline]
    fn is_writable_char(&self, char: &u8) -> bool {
        return *char >= 0x20 && *char <= 0x7e;
    }

    pub fn write_safe_string(&self, string: &str) {
        for char in string.bytes() {
            if char == b'\n' {
                self.write_u8(b'\r');
                self.write_u8(b'\n');
                continue;
            } else if self.is_writable_char(&char) {
                self.write_u8(char);
            } else {
                // invalid char byte
                self.write_u8(COM_INVALID_CHAR_BYTE);
            }
        }
    }
}

impl fmt::Write for UART {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        self.write_safe_string(string);
        return Ok(());
    }
}

fn init_uart() -> Option<Mutex<UART>> {
    if let Some(uart) = UART::new() {
        return Some(Mutex::new(uart));
    }
    None
}

lazy_static! {
    pub static ref UART_DRIVER: Option<Mutex<UART>> = init_uart();
}

// implement devfs operations:
pub struct UartIODriver;

impl UartIODriver {
    pub fn empty() -> Self {
        UartIODriver {}
    }
}

impl DevOps for UartIODriver {
    fn read(&self, fd: &mut DevFSDescriptor, buffer: &mut [u8]) -> Result<usize, FSError> {
        // read till the end
        if UART_DRIVER.is_some() {
            let uart_lock = UART_DRIVER.as_ref().unwrap().lock();
            uart_lock.read_to_buffer(buffer);
            return Ok(buffer.len());
        }

        Err(FSError::DeviceNotFound)
    }

    fn write(&self, fd: &mut DevFSDescriptor, buffer: &[u8]) -> Result<usize, FSError> {
        // read till the end
        if UART_DRIVER.is_some() {
            let uart_lock = UART_DRIVER.as_ref().unwrap().lock();
            uart_lock.write_from_buffer(buffer);
            return Ok(buffer.len());
        }

        Err(FSError::DeviceNotFound)
    }

    fn ioctl(&self, _command: u8) -> Result<(), FSError> {
        // stub
        Ok(())
    }

    fn seek(&self, fd: &mut DevFSDescriptor, offset: u32) -> Result<(), FSError> {
        // stub: because seek is not possible for serial char devices.
        Ok(())
    }
}

// TODO: Find a best way to mitigate this
unsafe impl Sync for UartIODriver {}
unsafe impl Send for UartIODriver {}
