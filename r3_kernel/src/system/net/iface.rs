extern crate alloc;
extern crate lazy_static;
extern crate smoltcp;
extern crate spin;

use smoltcp::iface::EthernetInterface;
use smoltcp::iface::EthernetInterfaceBuilder;
use smoltcp::phy::Device;
use smoltcp::phy::{DeviceCapabilities, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::Result as NetResult;

use lazy_static::lazy_static;
use spin::Mutex;

use alloc::vec::Vec;

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

/// VirtualNetworkInterface plugs the physical device with smoltcp
pub struct VirtualNetworkDevice {}

impl<'a> Device<'a> for VirtualNetworkDevice {
    type TxToken = VirtualTx;
    type RxToken = VirtualRx;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        None
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        None
    }

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities::default()
    }
}

/// This trait is used by the device to ack the network interrupt
pub trait NetworkInterrupt {
    /// acknowledge interrupt
    fn ack(&mut self);
}

#[derive(Debug, Clone)]
pub enum PhyTransmissionErr {
    InterfaceError = 0,
    NoTxBuffer = 1,
}

/// the core trait implemented by physical network device driver
pub trait PhysicalNetworkDevice {
    /// get the buffer region where the next packet must be copied to
    fn get_current_tx_buffer(&mut self) -> Result<&'static mut [u8], PhyTransmissionErr>;

    /// call the transmit on device's side and wait for the hardware driver to return back
    fn transmit_and_wait(
        &mut self,
        buffer: &mut [u8],
        length: usize,
    ) -> Result<(), PhyTransmissionErr>;

    // 
}
