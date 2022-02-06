extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use crate::cpu::hw_interrupts;
use crate::drivers;

use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::Device;
use smoltcp::phy::{DeviceCapabilities, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address};
use smoltcp::Error as NetError;
use smoltcp::Result as NetResult;

use core::sync::atomic::AtomicU64;

use spin::Mutex;
use spin::Once;

const NET_DEFAULT_MTU: usize = 1500;

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

/// Smoltcp token type for Transmission
pub struct VirtualTx {}

/// Smoltcp token type for Reception
pub struct VirtualRx {
    /// recv_buffer is a vector view over the DMA slice of the packet
    pub recv_buffer: Vec<u8>,
}

impl TxToken for VirtualTx {
    #[allow(unused_mut)]
    fn consume<R, F>(mut self, _timestamp: Instant, len: usize, f: F) -> NetResult<R>
    where
        F: FnOnce(&mut [u8]) -> NetResult<R>,
    {
        let mut phy_lock = PHY_ETHERNET_DRIVER.lock();
        if phy_lock.is_none() {
            log::error!("no physical interface found.");
            return Err(NetError::Illegal);
        }

        let phy_dev = phy_lock.as_mut().unwrap();
        let buffer_res = phy_dev.get_current_tx_buffer();
        if buffer_res.is_err() {
            log::error!("interface error: {:?}", buffer_res.unwrap_err());
            return Err(NetError::Illegal);
        }

        let buffer = buffer_res.unwrap();
        let buffer_copy_res = f(buffer);
        if buffer_copy_res.is_err() {
            log::error!("interface error: failed to copy packet to DMA buffer");
            return Err(NetError::Illegal);
        }

        let transmit_result = phy_dev.transmit_and_wait(buffer, len);
        if transmit_result.is_err() {
            log::error!("interface error: {:?}", transmit_result.unwrap_err());
            return Err(NetError::Illegal);
        }

        Ok(buffer_copy_res.unwrap())
    }
}

impl RxToken for VirtualRx {
    #[allow(unused_mut)]
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> NetResult<R>
    where
        F: FnOnce(&mut [u8]) -> NetResult<R>,
    {
        f(&mut self.recv_buffer)
    }
}

/// stores the stats of the network interface
pub struct VirtualNetworkDeviceStats {
    pub n_tx_packets: AtomicU64,
    pub n_tx_bytes: AtomicU64,
    pub n_rx_packets: AtomicU64,
    pub n_rx_bytes: AtomicU64,
}

pub type PhyNetDevType = dyn PhysicalNetworkDevice + Sync + Send;
pub type EthernetInterfaceType = EthernetInterface<'static, VirtualNetworkDevice>;

/// VirtualNetworkInterface plugs the physical device with smoltcp
pub struct VirtualNetworkDevice;

impl<'a> Device<'a> for VirtualNetworkDevice {
    type TxToken = VirtualTx;
    type RxToken = VirtualRx;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        None
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(VirtualTx {})
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();

        let dev_lock = PHY_ETHERNET_DRIVER.lock();

        caps.max_transmission_unit = if dev_lock.is_none() {
            NET_DEFAULT_MTU
        } else {
            let mtu_res = dev_lock.as_ref().unwrap().get_mtu_size();
            let dev_mtu = if mtu_res.is_ok() {
                log::debug!(
                    "MTU not provided by the device, using default mtu={}",
                    NET_DEFAULT_MTU
                );
                mtu_res.unwrap()
            } else {
                NET_DEFAULT_MTU
            };
            dev_mtu
        };

        caps.max_burst_size = Some(1);

        caps
    }
}

#[derive(Debug, Clone)]
pub enum PhyNetdevError {
    NoPhysicalDevice,
    NoTxBuffer,
    NoInterruptLine,
    NoMTU,
    InterruptHandlingError,
    EmptyInterruptRecvBuffer,
    InvalidRecvHeader,
}

/// the core trait implemented by physical network device driver
pub trait PhysicalNetworkDevice {
    /// get the buffer region where the next packet must be copied to
    fn get_current_tx_buffer(&mut self) -> Result<&'static mut [u8], PhyNetdevError>;

    /// call the transmit on device's side and wait for the hardware driver to return back
    fn transmit_and_wait(&mut self, buffer: &mut [u8], length: usize)
        -> Result<(), PhyNetdevError>;

    /// get the interrupt handler details from the network device
    fn handle_interrupt(&mut self) -> Result<(), PhyNetdevError>;

    /// get device interrupt line no
    fn get_interrupt_no(&self) -> Result<usize, PhyNetdevError>;

    /// get the device MTU size
    fn get_mtu_size(&self) -> Result<usize, PhyNetdevError>;

    /// get the mac address
    fn get_mac_address(&self) -> Result<[u8; 6], PhyNetdevError>;
}

/// the function will be called from the device's receiver function
/// triggered by the network interrupt and there is a frame in DMA buffer.
/// The parameter `buffer` contains the read-only slice view of the
/// DMA buffer.
pub fn handle_recv_packet(buffer: &[u8]) {
    log::debug!("{:?}", buffer);
}

/// contains the physical network device type, can be None, if `None`, loopback will be used.
static PHY_ETHERNET_DRIVER: Mutex<Option<Box<PhyNetDevType>>> = Mutex::new(None);
static ETHERNET_INTERFACE: Once<Mutex<EthernetInterfaceType>> = Once::new();

pub fn setup_network_interface() {
    // 1. get available network device:
    let device_opt = drivers::get_network_device();
    if device_opt.is_none() {
        log::error!("no network interfaces found, configuring interface in loopback mode.");
        // TODO: setup loopback
        return;
    }

    let netdev = device_opt.unwrap();

    // register device interrupt
    let interrupt_no = netdev.as_ref().get_interrupt_no().unwrap();
    hw_interrupts::register_network_interrupt(interrupt_no);

    if let Ok(mac_addr) = netdev.get_mac_address() {
        // TODO: Update this later
        let neighbor_cache = NeighborCache::new(BTreeMap::new());
        let routes = Routes::new(BTreeMap::new());
        let ip_addrs = [IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0)];
        let iface = EthernetInterfaceBuilder::new(VirtualNetworkDevice)
            .ethernet_addr(EthernetAddress::from_bytes(&mac_addr))
            .neighbor_cache(neighbor_cache)
            .ip_addrs(ip_addrs)
            .routes(routes)
            .finalize();

        log::info!(
            "Initialized system network interface, ip_addr={:?}",
            iface.ipv4_address()
        );
        ETHERNET_INTERFACE.call_once(|| Mutex::new(iface));
    }

    // save network device:
    *PHY_ETHERNET_DRIVER.lock() = Some(netdev);
}

pub fn network_interrupt_handler() {
    let mut net_dev_lock = PHY_ETHERNET_DRIVER.lock();
    if net_dev_lock.is_some() {
        let result = net_dev_lock.as_mut().unwrap().handle_interrupt();
        if result.is_err() {
            log::error!(
                "failed to handle device interrupt: {:?}",
                result.unwrap_err()
            );
        }
    }
}
