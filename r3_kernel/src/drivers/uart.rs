use crate::cpu::io::Port;

// on x86 architecture, 0x3f8 is the COM port.
const x86_COM_PORT: usize = 0x3f8;
const x86_COM_PORT_EMPTY_FLAG: usize = 0x20;
const x86_COM_PORT_RECEIVED_FLAG: usize = 0x1;
const x86_COM_PORT_LOOPBACK_MODE: usize = 0x1b;

pub struct UART {
    pub port_0: Port, // read write on this port
    pub port_5: Port, // check transit empty/recv on this port
}

impl UART {
    // set UART in loopback mode and check if same data written is read
    // as it is, if not, the chip is faulty.
    fn chip_works_fine(port_4: &Port, port_0: &Port) -> bool {
        port_4.write_u8(x86_COM_PORT_LOOPBACK_MODE as u8);
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
        let port_0 = Port::new(x86_COM_PORT, false);
        let port_1 = Port::new(x86_COM_PORT + 1, false);
        let port_2 = Port::new(x86_COM_PORT + 2, false);
        let port_3 = Port::new(x86_COM_PORT + 3, false);
        let port_4 = Port::new(x86_COM_PORT + 4, false);

        // read only port.
        let port_5 = Port::new(x86_COM_PORT + 5, true);

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
}
