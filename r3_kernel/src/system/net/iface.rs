extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use crate::cpu::hw_interrupts;
use crate::drivers;
use crate::system::net::ip_utils;
use crate::system::net::types;

use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::Device;
use smoltcp::phy::{DeviceCapabilities, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::Error as NetError;
use smoltcp::Result as NetResult;

use core::sync::atomic::AtomicU64;

use spin::Mutex;

const NET_DEFAULT_MTU: usize = 1500;

// TODO: Make these as variables passed from boot-info
const DEFAULT_GATEWAY: &str = "192.168.0.1";
const DEFAULT_STATIC_IP: &str = "192.168.0.24/24";

use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};

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
        log::debug!("called tx");
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

        let buffer = &mut buffer_res.unwrap()[0..len];
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
        log::debug!("called rx");
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
        log::debug!("called receive!");
        let mut phy_dev_lock = PHY_ETHERNET_DRIVER.lock();

        if let Some(phy_dev) = phy_dev_lock.as_mut() {
            if let Ok(true) = phy_dev.is_polling_enabled() {
                // poll for frame
                let poll_result = phy_dev.poll_for_frame();
                if let Ok(buffer) = poll_result {
                    let recv_buffer = buffer.to_vec();
                    return Some((VirtualRx { recv_buffer }, VirtualTx {}));
                }
            } else {
                if let Ok(recv_buffer) = types::NETWORK_IFACE_QUEUE.lock().pop() {
                    return Some((VirtualRx { recv_buffer }, VirtualTx {}));
                }
            }
        }
        None
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        log::debug!("called transmit!");
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
    PollingModeError,
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

    /// set polling mode
    fn set_polling_mode(&mut self, enable: bool) -> Result<(), PhyNetdevError>;

    /// is polling enabled?
    fn is_polling_enabled(&self) -> Result<bool, PhyNetdevError>;

    /// poll for packet
    fn poll_for_frame(&mut self) -> Result<&'static [u8], PhyNetdevError>;
}

/// the function will be called from the device's receiver function
/// triggered by the network interrupt and there is a frame in DMA buffer.
/// The parameter `buffer` contains the read-only slice view of the
/// DMA buffer.
pub fn handle_recv_packet(buffer: &[u8]) {
    let packet_vec = buffer.to_vec();
    log::debug!("received packet!");
    if let Err(_) = types::NETWORK_IFACE_QUEUE.lock().push(packet_vec) {
        log::debug!("dropping network packet because interface queue is full")
    }
}

/// contains the physical network device type, can be None, if `None`, loopback will be used.
static PHY_ETHERNET_DRIVER: Mutex<Option<Box<PhyNetDevType>>> = Mutex::new(None);
pub static ETHERNET_INTERFACE: Mutex<Option<EthernetInterfaceType>> = Mutex::new(None);

fn create_unspecified_interface(mac_addr: &[u8]) -> EthernetInterfaceType {
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let routes = Routes::new(BTreeMap::new());
    let ip_addrs = [IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0)];
    let iface = EthernetInterfaceBuilder::new(VirtualNetworkDevice)
        .ethernet_addr(EthernetAddress::from_bytes(&mac_addr))
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();
    iface
}

fn create_static_ip_interface(
    mac: &[u8],
    gateway: &str,
    ip: &str,
) -> Option<EthernetInterfaceType> {
    let mut routes = Routes::new(BTreeMap::new());
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    // set gateway as the default route:
    let gateway_ip = ip_utils::get_ipv4_from_string(gateway)?;

    routes
        .add_default_ipv4_route(gateway_ip)
        .expect("failed to add default ip to the network routes");
    // set provided static IP
    let (ip, prefix) = ip_utils::get_ipv4_with_prefix_from_string(ip)?;
    let ip_addrs = [IpCidr::new(IpAddress::from(ip), prefix as u8)];

    log::info!(
        "creating static IP interface ip={}, gateway={}",
        DEFAULT_STATIC_IP,
        DEFAULT_GATEWAY
    );

    // create the ethernet interface
    let iface = EthernetInterfaceBuilder::new(VirtualNetworkDevice)
        .ethernet_addr(EthernetAddress::from_bytes(mac))
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    log::debug!("created interface");
    Some(iface)
}

fn create_loopback_interface() -> EthernetInterfaceType {
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let routes = Routes::new(BTreeMap::new());

    let (ip, prefix) = ip_utils::get_ipv4_with_prefix_from_string("127.0.0.1/8").unwrap();
    let ip_addrs = [IpCidr::new(IpAddress::from(ip), prefix as u8)];
    let iface = EthernetInterfaceBuilder::new(VirtualNetworkDevice)
        .ethernet_addr(EthernetAddress::default().into())
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();
    iface
}

pub fn setup_network_interface() {
    // 1. get available network device:
    let device_opt = drivers::get_network_device();
    if device_opt.is_none() {
        log::error!("no network interfaces found, configuring interface in loopback mode.");
        let iface = create_loopback_interface();
        log::info!(
            "Initialized system network interface, ip_addr={:?}",
            iface.ipv4_address()
        );

        *ETHERNET_INTERFACE.lock() = Some(iface);
        return;
    }

    let mut netdev = device_opt.unwrap();

    // TODO: FIX interrupt mode bugs - interrupts not firing as of now
    netdev
        .set_polling_mode(true)
        .expect("failed to enable polling mode on ethernet device");

    let interrupt_no = netdev.as_ref().get_interrupt_no().unwrap();

    hw_interrupts::register_network_interrupt(interrupt_no);
    log::info!("registered network interrupt on line: {}", interrupt_no);

    *PHY_ETHERNET_DRIVER.lock() = Some(netdev);

    let mac_result = PHY_ETHERNET_DRIVER
        .lock()
        .as_ref()
        .unwrap()
        .get_mac_address();

    // register device interrupt
    if let Ok(mac_addr) = mac_result {
        // TODO: Update this later
        if let Some(iface) =
            create_static_ip_interface(&mac_addr, DEFAULT_GATEWAY, DEFAULT_STATIC_IP)
        {
            log::info!(
                "initialized network interface with IP: {:?}",
                iface.ipv4_address()
            );
            *ETHERNET_INTERFACE.lock() = Some(iface);
        } else {
            let iface = create_unspecified_interface(&mac_addr);
            log::warn!(
                "initialized network interface with unspecified IP: {:?}",
                iface.ipv4_address()
            );
            *ETHERNET_INTERFACE.lock() = Some(iface);
        }
    }

    types::setup_interface_queue();
}

pub fn network_interrupt_handler() {
    log::debug!("got network interrupt!");
    let mut net_dev_lock = PHY_ETHERNET_DRIVER.lock();
    if net_dev_lock.is_some() {
        let result = net_dev_lock.as_mut().unwrap().handle_interrupt();
        if result.is_err() {
            log::debug!(
                "failed to handle device interrupt: {:?}",
                result.unwrap_err()
            );
        }
    }
}

pub fn get_formatted_mac() -> Option<String> {
    let phy_dev_lock = PHY_ETHERNET_DRIVER.lock();
    if let Ok(mac_bytes) = phy_dev_lock.as_ref().unwrap().get_mac_address() {
        let hexified: Vec<String> = mac_bytes
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();
        let stringified_mac = hexified.join(":");
        return Some(stringified_mac);
    }

    None
}
