use crate::cpu::io::Port;

// Serial port interface on given io::Port object.
pub struct Serial {
    pub port: Port,
}

// The serial port used to send and receive bytes.
const SERIAL_PORT: usize = 0x3f8;

impl Serial {

    // create a serial port interface on the serial port.
    pub fn new() -> Self {
        Serial {
            port: Port::new(SERIAL_PORT, false),
        }
    }

    // writes a single byte through the serial port.
    // byte: the byte to be written. 
    pub fn write_u8(&self, byte: u8) {
        self.port.write_u8(byte);
    }

    // reads a single byte and returns it from the serial port.
    pub fn read_u8(&self) -> u8 {
        self.port.read_u8()
    }

    // writes the contents of the buffer one byte at a time,
    // SerialIO is a char device so it wont't support writing a block at a time.
    pub fn write_buffer(&self, buffer: &[u8]) {
        for byte in buffer {
            self.port.write_u8(*byte);
        }
    }

    // reads the `size` amount of bytes from serial port to a buffer.
    // Since SerialIO is a chardevice, one byte is read at once.
    pub fn read_buffer(&self, buffer: &mut [u8], size: usize) {
        for index in 0..size {
            let read_char = self.port.read_u8();
            buffer[index] = read_char;
        }
    }
}
