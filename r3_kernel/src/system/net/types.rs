extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use alloc::{collections::BTreeSet, vec, vec::Vec};
use lazy_static::lazy_static;
use smoltcp::socket::SocketSet;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};
use spin::Mutex;

const MAX_IFACE_QUEUE_SIZE: usize = 64;

type NetworkInterfacePacket = Vec<u8>;

pub enum NetworkInterfaceQueueError {
    QueueEmpty = 0,
    QueueFull = 1,
}

pub struct NetworkInterfaceQueue {
    pub queue: Vec<NetworkInterfacePacket>,
}

pub static SOCKETS_SET: Mutex<Option<SocketSet>> = Mutex::new(None);

lazy_static! {
    pub static ref NETWORK_IFACE_QUEUE: Mutex<NetworkInterfaceQueue> =
        Mutex::new(NetworkInterfaceQueue::new());
}

impl NetworkInterfaceQueue {
    pub fn new() -> Self {
        Self {
            queue: Vec::with_capacity(MAX_IFACE_QUEUE_SIZE),
        }
    }

    #[inline]
    pub fn push(&mut self, data: NetworkInterfacePacket) -> Result<(), NetworkInterfaceQueueError> {
        if self.queue.len() < MAX_IFACE_QUEUE_SIZE {
            self.queue.push(data);
            return Ok(());
        }

        Err(NetworkInterfaceQueueError::QueueFull)
    }

    #[inline]
    pub fn pop(&mut self) -> Result<NetworkInterfacePacket, NetworkInterfaceQueueError> {
        if let Some(data) = self.queue.pop() {
            return Ok(data);
        }

        Err(NetworkInterfaceQueueError::QueueEmpty)
    }
}

pub fn setup_interface_queue() {
    log::info!(
        "initialized network interface queue with size={}",
        NETWORK_IFACE_QUEUE.lock().queue.capacity()
    );
}

pub fn setup_socket_set() {
    *SOCKETS_SET.lock() = Some(SocketSet::new(vec![]));
}

pub type TransportLayerPort = u16;
pub type TransportLayerPorts = BTreeSet<TransportLayerPort>;
pub type TransportSocketFlags = u16;

#[derive(Debug, Clone)]
pub enum TransportType {
    // TODO: Support IPv6 network backend
    AFUnix = 1,
    AFInet = 2,
}

#[derive(Debug, Clone)]
pub enum TransportSocketTypes {
    SockStream = 1,
    SockDgram = 2,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct NetworkSocketAddress {
    family: TransportSocketFlags,
    port: [u8; 2],
    address: [u8; 4],
    padding: [u8; 8],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UnixSocketAddress {
    // TODO
}

#[derive(Debug, Clone, Copy)]
pub enum SocketAddr {
    Network(NetworkSocketAddress),
    Unix(UnixSocketAddress),
}

impl SocketAddr {
    #[inline]
    pub fn from_inet_addr(ep: &IpEndpoint) -> SocketAddr {
        let ip_addr = match ep.addr {
            IpAddress::Ipv4(addr) => addr.0,
            // TODO: Support IPv6 network backend, ipv6 address will be treated
            // as unspecified as of now.
            _ => Ipv4Address::UNSPECIFIED.0,
        };

        let sock_port = ep.port.to_be_bytes();

        SocketAddr::Network(NetworkSocketAddress {
            family: TransportType::AFInet as u16,
            address: ip_addr,
            port: sock_port,
            padding: [0; 8],
        })
    }

    #[inline]
    pub fn to_inet_addr(&self) -> Option<IpEndpoint> {
        let ep_addr = match self {
            SocketAddr::Network(sock_addr) => {
                let port = u16::from_be_bytes(sock_addr.port);

                let addr = if u32::from_be_bytes(sock_addr.address) == 0 {
                    IpAddress::Unspecified
                } else {
                    IpAddress::Ipv4(Ipv4Address::from_bytes(&sock_addr.address))
                };

                Some(IpEndpoint { addr, port })
            }
            SocketAddr::Unix(_) => None,
        };

        ep_addr
    }
}

lazy_static! {
    pub static ref CURRENT_TL_PORTS: Mutex<TransportLayerPorts> =
        Mutex::new(TransportLayerPorts::new());
}
