extern crate spin;

use crate::cpu::io::{wait, Port};
use crate::drivers::pci;
use crate::mm::phy;
use crate::system::net::iface;

use lazy_static::lazy_static;

use spin::Mutex;

const RTL_VENDOR_ID: u16 = 0x10EC;
const RTL_DEVICE_ID: u16 = 0x8139;

// RTL packet type falgs
const RTL_ENABLE_ALL_PACKETS: usize = 1 << 0;
const RTL_ENABLE_MATCH_PACKETS: usize = 1 << 1;
const RTL_ENABLE_MULTICAST: usize = 1 << 2;
const RTL_ENABLE_BROADCAST: usize = 1 << 3;

// RTL DMA flags
const RTL_TX_DMA0: usize = 1 << 8;
const RTL_TX_DMA1: usize = 1 << 9;
const RTL_TX_DMA2: usize = 1 << 10;

// RTL basic device flags
const RTL_RECV_OK: usize = 0x01;
const RTL_TX_OK: usize = 1 << 15;
const RTL_DMA_COMPLETE: usize = 1 << 13;

const RTL_WRAP_BUFFER: usize = 1 << 7;
const RTL_INTERFRAME_TIME_GAP: usize = 1 << 24;
const RTL_RX_BUFFER_PAD: usize = 16;
const RTL_RX_BUFFER_LENGTH: usize = 0 << 11;
const RTL_TX_BUFFER_SIZE: usize = 4096;
const RTL_RX_BUFFER_SIZE: usize = 8145;
const PHY_MTU_SIZE: usize = 1500;
const RTL_N_TX_BUFFERS: usize = 4;

/// First 13 bits will be used for representing length
const RTL_LENGTH_BITS: usize = 0x1FFF;

// RTL interrupts flags
const RTL_INTERRUPT_RECVOK: usize = 1 << 0;
const RTL_INTERRUPT_TXOK: usize = 1 << 2;

#[repr(u8)]
pub enum RTLDeviceCommand {
    HardPowerUp = 0,
    SoftReset = 1 << 4,
    EnableTx = 1 << 2,
    EnableRx = 1 << 3,
    EmptyBuffer = 1 << 0,
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
    tx_dma: [phy::DMABuffer; RTL_N_TX_BUFFERS],
    rx_dma: phy::DMABuffer,
}

impl DeviceBuffers {
    #[inline]
    pub fn new() -> Self {
        let rx_dma = phy::DMAMemoryManager::alloc(RTL_RX_BUFFER_SIZE + PHY_MTU_SIZE)
            .expect("Failed to allocate DMA buffer");

        let tx_dma: [phy::DMABuffer; RTL_N_TX_BUFFERS] = [(); RTL_N_TX_BUFFERS].map(|_| {
            phy::DMAMemoryManager::alloc(RTL_TX_BUFFER_SIZE).expect("Failed to allocate DMA buffer")
        });

        DeviceBuffers { rx_dma, tx_dma }
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

    #[inline]
    fn configure_receiver(&self) {
        let recv_buffer_addr = self.buffers.rx_dma.phy_addr.as_u64() as u32;
        self.rx_line.addr.write_u32(recv_buffer_addr);
    }

    #[inline]
    fn configure_transmitter(&self) {
        for idx in 0..RTL_N_TX_BUFFERS {
            let tx_buffer_addr = self.buffers.tx_dma[idx].phy_addr.as_u64() as u32;
            self.tx_line.addr[idx].write_u32(tx_buffer_addr);
        }
    }

    #[inline]
    fn finalize_config(&self) {
        // configure interrupts:
        self.config
            .imr
            .write_u32((RTL_INTERRUPT_TXOK | RTL_INTERRUPT_RECVOK) as u32);
        // setup operation modes of buffers
        self.rx_line.config.write_u32(
            (RTL_RX_BUFFER_LENGTH
                | RTL_WRAP_BUFFER
                | RTL_ENABLE_ALL_PACKETS
                | RTL_ENABLE_MATCH_PACKETS
                | RTL_ENABLE_MULTICAST
                | RTL_ENABLE_BROADCAST) as u32,
        );

        self.tx_line
            .config
            .write_u32((RTL_INTERFRAME_TIME_GAP | RTL_TX_DMA0 | RTL_TX_DMA1 | RTL_TX_DMA2) as u32);
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

        log::debug!("configuring transmitter and receivers");
        self.configure_transmitter();
        self.configure_receiver();
        self.finalize_config();
    }
}

impl iface::PhysicalNetworkDevice for Realtek8139Device {
    fn get_current_tx_buffer(&mut self) -> Result<&'static mut [u8], iface::PhyTransmissionErr> {
        let tx_id = self.tx_line.tx_id;
        if tx_id >= RTL_N_TX_BUFFERS {
            return Err(iface::PhyTransmissionErr::NoTxBuffer);
        }

        Ok(self.buffers.tx_dma[tx_id].get_mut_slice::<u8>())
    }

    fn transmit_and_wait(
        &mut self,
        _buffer: &mut [u8],
        length: usize,
    ) -> Result<(), iface::PhyTransmissionErr> {
        let tx_id = self.tx_line.tx_id;
        if tx_id >= RTL_N_TX_BUFFERS {
            return Err(iface::PhyTransmissionErr::NoTxBuffer);
        }

        // get current command port
        let tx_cmd_port = self.tx_line.cmds[tx_id];
        // write the length:
        tx_cmd_port.write_u32((RTL_LENGTH_BITS & length) as u32);
        // wait for packet to be moved from DMA to FIFO queue
        while (tx_cmd_port.read_u32() as usize & RTL_DMA_COMPLETE) != RTL_DMA_COMPLETE {}
        // wait for Tx to complete
        while (tx_cmd_port.read_u32() as usize & RTL_TX_OK) != RTL_TX_OK {}

        // increment tx_id
        self.tx_line.tx_id = (tx_id + 1) % RTL_N_TX_BUFFERS;

        // Tx is not complete
        Ok(())
    }
}
