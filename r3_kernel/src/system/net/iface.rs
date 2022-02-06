extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use smoltcp::iface::EthernetInterface;
use smoltcp::iface::EthernetInterfaceBuilder;
use smoltcp::phy::Device;
use smoltcp::phy::{DeviceCapabilities, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::Result as NetResult;

use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

const NET_DEFAULT_MTU: usize = 1500;

use alloc::{boxed::Box, vec::Vec};

/// Smoltcp token type for Transmission
pub struct VirtualTx {}

/// Smoltcp token type for Reception
pub struct VirtualRx {
    /// recv_buffer is a vector view over the DMA slice of the packet
    pub recv_buffer: Vec<u8>,
}

impl TxToken for VirtualTx {
    fn consume<R, F>(mut self, timestamp: Instant, len: usize, f: F) -> NetResult<R>
    where
        F: FnOnce(&mut [u8]) -> NetResult<R>,
    {
        let mut buff: [u8; 10] = [0; 10];
        f(&mut buff)
    }
}

impl RxToken for VirtualRx {
    fn consume<R, F>(mut self, timestamp: Instant, f: F) -> NetResult<R>
    where
        F: FnOnce(&mut [u8]) -> NetResult<R>,
    {
        let mut buff: [u8; 10] = [0; 10];
        f(&mut buff)
    }
}

/// stores the stats of the network interface
pub struct VirtualNetworkDeviceStats {
    pub n_tx_packets: AtomicU64,
    pub n_tx_bytes: AtomicU64,
    pub n_rx_packets: AtomicU64,
    pub n_rx_bytes: AtomicU64,
}

type PhyNetDevType = dyn PhysicalNetworkDevice + Sync + Send;

/// VirtualNetworkInterface plugs the physical device with smoltcp
pub struct VirtualNetworkDevice {
    /// represents a physical network device
    /// this can be optional, if `None`, the loopback interface
    /// will be used with `127.0.0.1` address.
    phy_driver: Option<Box<PhyNetDevType>>,
    stats: VirtualNetworkDeviceStats,
}

impl VirtualNetworkDevice {
    pub fn with_mut_phy_dev_ref<F, R>(&mut self, mut virtual_func: F) -> Result<R, PhyNetdevError>
    where
        F: FnMut(&mut PhyNetDevType) -> Result<R, PhyNetdevError>,
    {
        if self.phy_driver.is_none() {
            return Err(PhyNetdevError::NoPhysicalDevice);
        }

        let dev_mut_ref = self.phy_driver.as_mut().unwrap().as_mut();
        virtual_func(dev_mut_ref)
    }
}

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

        caps.max_transmission_unit = if self.phy_driver.is_none() {
            NET_DEFAULT_MTU
        } else {
            let mtu_res = self.phy_driver.as_ref().unwrap().get_mtu_size();
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
}

/// the function will be called from the device's receiver function
/// triggered by the network interrupt and there is a frame in DMA buffer.
/// The parameter `buffer` contains the read-only slice view of the
/// DMA buffer.
pub fn handle_recv_packet(_buffer: &[u8]) {}

pub fn setup_network_interface() {}
