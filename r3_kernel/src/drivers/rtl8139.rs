extern crate spin;

use crate::cpu::io::{wait, Port};
use crate::drivers::pci;
use crate::mm::phy;

use lazy_static::lazy_static;

use spin::Mutex;

const RTL_TX_BUFFER_SIZE: usize = 4096;
const RTL_RX_BUFFER_SIZE: usize = 8129 + 16;
const PHY_MTU_SIZE: usize = 1500;
const RTL_VENDOR_ID: u16 = 0x10EC;
const RTL_DEVICE_ID: u16 = 0x8139;

#[repr(u8)]
pub enum RTLDeviceCommand {
    HardPowerUp = 0,
    SoftReset = 1 << 4,
    EnableTx = 1 << 2,
    EnableRx = 1 << 3,
}

struct DeviceTx {
    cmds: [Port; 4],
    addr: [Port; 4],
    config: Port,
    tx_id: usize,
}

impl DeviceTx {
    #[inline]
    pub fn new(io_base: usize) -> DeviceTx {
        DeviceTx {
            cmds: [
                Port::new(io_base + 0x10, false),
                Port::new(io_base + 0x14, false),
                Port::new(io_base + 0x18, false),
                Port::new(io_base + 0x1C, false),
            ],
            addr: [
                Port::new(io_base + 0x20, false),
                Port::new(io_base + 0x24, false),
                Port::new(io_base + 0x28, false),
                Port::new(io_base + 0x2C, false),
            ],
            config: Port::new(io_base + 0x040, false),
            tx_id: 0,
        }
    }
}

struct DeviceConfig {
    config_1: Port,
    capr: Port,
    cbr: Port,
    cmd: Port,
    imr: Port,
    isr: Port,
}

impl DeviceConfig {
    #[inline]
    pub fn new(io_base: usize) -> Self {
        DeviceConfig {
            config_1: Port::new(io_base + 0x52, false),
            capr: Port::new(io_base + 0x38, false),
            cbr: Port::new(io_base + 0x3A, false),
            cmd: Port::new(io_base + 0x37, false),
            imr: Port::new(io_base + 0x3C, false),
            isr: Port::new(io_base + 0x3E, false),
        }
    }
}

struct DeviceRx {
    addr: Port,
    config: Port,
}

impl DeviceRx {
    #[inline]
    pub fn new(io_base: usize) -> Self {
        DeviceRx {
            addr: Port::new(io_base + 0x30, false),
            config: Port::new(io_base + 0x44, false),
        }
    }
}

struct DeviceBuffers {
    tx_dma_8k: phy::DMABuffer,
    tx_dma_16k: phy::DMABuffer,
    tx_dma_32k: phy::DMABuffer,
    tx_dma_64k: phy::DMABuffer,
    rx_dma: phy::DMABuffer,
}

impl DeviceBuffers {
    #[inline]
    pub fn new() -> Self {
        DeviceBuffers {
            tx_dma_8k: phy::DMAMemoryManager::alloc(RTL_TX_BUFFER_SIZE)
                .expect("Failed to allocate DMA buffer"),
            tx_dma_16k: phy::DMAMemoryManager::alloc(RTL_TX_BUFFER_SIZE)
                .expect("Failed to allocate DMA buffer"),
            tx_dma_32k: phy::DMAMemoryManager::alloc(RTL_TX_BUFFER_SIZE)
                .expect("Failed to allocate DMA buffer"),
            tx_dma_64k: phy::DMAMemoryManager::alloc(RTL_TX_BUFFER_SIZE)
                .expect("Failed to allocate DMA buffer"),
            rx_dma: phy::DMAMemoryManager::alloc(RTL_RX_BUFFER_SIZE + PHY_MTU_SIZE)
                .expect("Failed to allocate DMA buffer"),
        }
    }
}

struct DeviceMAC {
    pub ports: [Port; 6],
}

impl DeviceMAC {
    #[inline]
    pub fn new(io_base: usize) -> Self {
        DeviceMAC {
            ports: [
                Port::new(io_base + 0, true),
                Port::new(io_base + 1, true),
                Port::new(io_base + 2, true),
                Port::new(io_base + 3, true),
                Port::new(io_base + 4, true),
                Port::new(io_base + 5, true),
            ],
        }
    }

    #[inline]
    pub fn get_mac(&self) -> [u8; 6] {
        [
            self.ports[0].read_u8(),
            self.ports[1].read_u8(),
            self.ports[2].read_u8(),
            self.ports[3].read_u8(),
            self.ports[4].read_u8(),
            self.ports[5].read_u8(),
        ]
    }
}

pub struct Realtek8139Device {
    tx_line: DeviceTx,
    rx_line: DeviceRx,
    buffers: DeviceBuffers,
    mac: DeviceMAC,
    config: DeviceConfig,
}

impl Realtek8139Device {
    #[inline]
    fn send_command(&self, port: &Port, command: u8) {
        port.write_u8(command)
    }

    #[inline]
    fn wait_soft_reset(&self) {
        self.send_command(&self.config.cmd, RTLDeviceCommand::SoftReset as u8);
        loop {
            let rst_value = self.config.cmd.read_u8();
            if rst_value & RTLDeviceCommand::SoftReset as u8 != 0 {
                wait(1);
                continue;
            }

            break;
        }
    }

    pub fn new() -> Realtek8139Device {
        let pci_dev = pci::search_device(RTL_VENDOR_ID, RTL_DEVICE_ID).unwrap();
        // enable bus mastering
        pci_dev.set_bus_mastering();

        // get io base register offset
        let io_base = (pci_dev.bars[0] & 0xFFF0) as usize;
        log::info!("Initialized RTL 8139 device driver, MAC address");
        Realtek8139Device {
            tx_line: DeviceTx::new(io_base),
            rx_line: DeviceRx::new(io_base),
            buffers: DeviceBuffers::new(),
            mac: DeviceMAC::new(io_base),
            config: DeviceConfig::new(io_base),
        }
    }

    fn configure_receiver() {

    }

    fn configure_transmitter() {
        
    }

    pub fn prepare_interface(&mut self) {
        // 1. boot up
        self.send_command(&self.config.config_1, RTLDeviceCommand::HardPowerUp as u8);
        // 2. soft-reset
        self.wait_soft_reset();
        // 3. Enable both tx and rx modes
        self.send_command(
            &self.config.cmd,
            RTLDeviceCommand::EnableTx as u8 | RTLDeviceCommand::EnableRx as u8,
        );

        // read mac
        let mac = self.mac.get_mac();


        log::info!("Initialized RTL 8139 device driver, MAC address: {:?}", mac);
    }
}

lazy_static! {
    pub static ref RTL_DEVICE: Mutex<Realtek8139Device> = Mutex::new(Realtek8139Device::new());
}

pub fn init() {
    RTL_DEVICE.lock().prepare_interface();
}
