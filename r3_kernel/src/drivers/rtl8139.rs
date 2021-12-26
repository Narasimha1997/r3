use crate::cpu::io::Port;
use crate::drivers::pci;
use crate::mm::phy;

const RTL_TX_BUFFER_SIZE: usize = 4096;
const RTL_RX_BUFFER_SIZE: usize = 8129 + 16;
const PHY_MTU_SIZE: usize = 1500;

pub struct Realtek8139Device {}

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

const RTL_VENDOR_ID: u16 = 0x10EC;
const RTL_DEVICE_ID: u16 = 0x8139;

impl Realtek8139Device {
    pub fn new() {
        let pci_dev = pci::search_device(RTL_VENDOR_ID, RTL_DEVICE_ID).unwrap();
        // enable bus mastering
        pci_dev.set_bus_mastering();

        // get io base register offset
        let io_base = pci_dev.bars[0] & 0xFFF0;
    }
}
