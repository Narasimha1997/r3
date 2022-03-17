extern crate smoltcp;
extern crate alloc;

use crate::system::net::{types, process::process_network_packet_event};
use crate::cpu;
use crate::system::tasking;

use smoltcp::socket;
use alloc::vec;

pub struct UDPSocket {
    sock_handle: socket::SocketHandle
}

const UDP_TX_BUFFER_LENGTH: usize = 4096;
const UDP_RX_BUFFER_LENGTH: usize = 4096;
const UDP_METADATA_LENGTH: usize = 64;

impl UDPSocket {
    /// creates a UDP socket with all the buffers but does not bind it to any port
    pub fn empty() -> UDPSocket {
        // allocate Rx Buffer
        let udp_rx_buf = socket::UdpSocketBuffer::new(
            vec![socket::UdpPacketMetadata::EMPTY; UDP_METADATA_LENGTH],
            vec![0; UDP_RX_BUFFER_LENGTH] 
        );

        let udp_tx_buf = socket::UdpSocketBuffer::new(
            vec![socket::UdpPacketMetadata::EMPTY; UDP_METADATA_LENGTH],
            vec![0; UDP_TX_BUFFER_LENGTH]
        );

        let socket = socket::UdpSocket::new(udp_rx_buf, udp_tx_buf);
        let sock_handle = types::SOCKETS_SET.lock().as_mut().unwrap().add(socket);
        UDPSocket { sock_handle } 
    }
}

impl types::SocketFn for UDPSocket {
    fn bind(&self, addr: types::SocketAddr) -> Result<(), types::SocketError> {
        let ip_endpoint_opt = addr.to_inet_addr();
        if ip_endpoint_opt.is_none() {
            return Err(types::SocketError::InvalidAddress);
        }

        let ip_endpoint = ip_endpoint_opt.unwrap();

        let mut current_endpoints = types::CURRENT_TL_PORTS.lock();

        if current_endpoints.contains(&ip_endpoint.port) {
            return Err(types::SocketError::PortAlreadyInUse);
        }

        // create a socket
        let mut sock_set_lock = types::SOCKETS_SET.lock();
        let sock_set = sock_set_lock.as_mut().unwrap();

        let mut udp_socket = sock_set.get::<socket::UdpSocket>(self.sock_handle);

        // bind to this port
        let bind_res = udp_socket.bind(ip_endpoint.port);
        if bind_res.is_err() {
            return Err(types::SocketError::BindError);
        }

        // add this port to endpoints list
        current_endpoints.insert(ip_endpoint.port);

        Ok(())
    }

    fn sendto(&self, addr: types::SocketAddr, buffer: &[u8]) -> Result<usize, types::SocketError> {
        let ip_endpoint_opt = addr.to_inet_addr();
        if ip_endpoint_opt.is_none() {
            return Err(types::SocketError::InvalidAddress);
        }

        let ip_endpoint = ip_endpoint_opt.unwrap();

        let mut sockets_lock = types::SOCKETS_SET.lock();
        let all_socks = sockets_lock.as_mut().unwrap();

        let mut udp_socket = all_socks.get::<socket::UdpSocket>(self.sock_handle);
        let send_res = udp_socket.send(buffer.len(), ip_endpoint);
        if send_res.is_err() {
            return Err(types::SocketError::SendError);
        }

        let dest_buffer_region = send_res.unwrap();
        dest_buffer_region.copy_from_slice(buffer);

        // release locks
        drop(udp_socket);
        drop(sockets_lock);

        log::debug!("sent!");

        // process packets
        process_network_packet_event();

        Ok(buffer.len())
    }

    fn recvfrom(&self, buffer: &mut [u8]) -> Result<(usize, types::SocketAddr), types::SocketError> {
        cpu::enable_interrupts();
        let wait_res = tasking::wait_until_return(|| {
            let mut sock_lock = types::SOCKETS_SET.lock();
            let all_socks = sock_lock.as_mut().unwrap();

            let mut udp_socket = all_socks.get::<socket::UdpSocket>(self.sock_handle);
            let recv_result = udp_socket.recv();
            match recv_result {
                Ok((payload, ip_endpoint)) => {
                    buffer[0..payload.len()].copy_from_slice(payload);
                    let sock_addr = types::SocketAddr::from_inet_addr(&ip_endpoint);
                    return Ok(Some((payload.len(), sock_addr)))
                }
                // this buffer is empty, No data
                Err(smoltcp::Error::Exhausted) => Ok(None),
                Err(_) => {
                    return Err(types::SocketError::RecvError)
                } 
            }
        });

        // handle the error
        wait_res
    }
}