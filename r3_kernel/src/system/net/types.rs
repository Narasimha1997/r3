extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use crate::mm;

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

lazy_static! {
    pub static ref CURRENT_TL_PORTS: Mutex<TransportLayerPorts> =
        Mutex::new(TransportLayerPorts::new());
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
    family: TransportSocketFlags
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

                // TODO: Support IPv6
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

    pub fn from_memory_view(vaddr: mm::VirtualAddress) -> Option<SocketAddr> {
        // first we will typecast this to 
        let sock_family: &TransportSocketFlags = unsafe { &*vaddr.get_ptr() };
        match sock_family {
            // AFInet
            2 => {
                let netsock_view: &NetworkSocketAddress = unsafe { &*vaddr.get_ptr() };
                let net_addr = NetworkSocketAddress {
                    family: *sock_family,
                    port: netsock_view.port,
                    address: netsock_view.address,
                    padding: [0; 8]
                };

                return Some(SocketAddr::Network(net_addr));
            }
            1 => {
                // TODO:
                let unix_addr = UnixSocketAddress {
                    family: *sock_family
                };

                return Some(SocketAddr::Unix(unix_addr));
            }
            _ => {
                return None;
            }
        }
    }

    pub fn write_to_memory(&self, vaddr: mm::VirtualAddress) {
        // writes self to memory
        match self {
            SocketAddr::Network(net_addr) => {
                let mem_view: &mut NetworkSocketAddress = unsafe { &mut *vaddr.get_mut_ptr() };
                mem_view.family = net_addr.family;
                mem_view.padding = net_addr.padding;
                mem_view.port = net_addr.port;
                mem_view.address = net_addr.address;
            }
            SocketAddr::Unix(unix_addr) => {
                let mem_view: &mut UnixSocketAddress = unsafe { &mut *vaddr.get_mut_ptr() };
                mem_view.family = unix_addr.family;
            }
        }
    }
}

#[derive(Debug)]
pub enum SocketError {
    InvalidAddress,
    PortAlreadyInUse,
    SendError,
    WIP,
    BindError,
}

pub trait SocketFn {
    /// bind socket to specified address, throw SocketError if not possible
    fn bind(&self, addr: SocketAddr) -> Result<(), SocketError>;
    /// send data to the destination address, throw SocketError if not possible 
    fn sendto(&self, addr: SocketAddr, buffer: &[u8]) -> Result<usize, SocketError>;
    /// receive data from the destination address, throw SocketError if not possible 
    fn recvfrom(&self, addr: SocketAddr, buffer: &[u8]) -> Result<usize, SocketError>; 
}
