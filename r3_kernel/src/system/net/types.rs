extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use alloc::{vec::Vec, vec};
use lazy_static::lazy_static;
use smoltcp::socket::SocketSet;
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