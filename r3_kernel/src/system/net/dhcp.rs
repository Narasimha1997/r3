extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use alloc::vec;
use lazy_static::lazy_static;
use smoltcp::dhcp::Dhcpv4Client;
use smoltcp::socket::{RawPacketMetadata, RawSocketBuffer};
use smoltcp::time::Instant;
use spin::Mutex;

use crate::system::net::iface;
use crate::system::net::types::SOCKETS_SET;
use crate::system::timer::PosixTimeval;

const DHCP_BUFFER_SIZE: usize = 2048;

lazy_static! {
    pub static ref DHCP_CLIENT: Mutex<Option<Dhcpv4Client>> = Mutex::new(None);
}

pub struct DHCPClient;

impl DHCPClient {
    pub fn init() {
        let dhcp_rx = RawSocketBuffer::new([RawPacketMetadata::EMPTY], vec![0; DHCP_BUFFER_SIZE]);
        let dhcp_tx = RawSocketBuffer::new([RawPacketMetadata::EMPTY], vec![0; DHCP_BUFFER_SIZE]);

        // create a DHCP client from these sockets:
        let ts = PosixTimeval::from_ticks().mills();
        let instance = Instant::from_millis(ts as i64);

        let mut sockets_lock = SOCKETS_SET.lock();
        let mut sockets = sockets_lock.as_mut().unwrap();

        let dhcp = Dhcpv4Client::new(&mut sockets, dhcp_rx, dhcp_tx, instance);
        *DHCP_CLIENT.lock() = Some(dhcp);

        log::info!("initialized DHCPv4 client")
    }

    pub fn poll_dhcp_over_iface() {
        let mut iface_lock = iface::ETHERNET_INTERFACE.lock();
        if iface_lock.as_ref().is_none() {
            log::error!("cannot poll DHCP over empty interface");
            return;
        }

        let mut iface = iface_lock.as_mut().unwrap();
        let mut dhcp_lock = DHCP_CLIENT.lock();

        if dhcp_lock.is_none() {
            log::error!("cannot poll over empty DHCP client");
        }
        let dhcp = dhcp_lock.as_mut().unwrap();
        let ts = PosixTimeval::from_ticks().mills();
        let instant = Instant::from_millis(ts as i64);

        let mut sockets_lock = SOCKETS_SET.lock();
        let mut sockets = sockets_lock.as_mut().unwrap();

        loop {
            let poll_result = dhcp.poll(&mut iface, &mut sockets, instant);
            if poll_result.is_err() {
                log::error!("DHCP poll error: {:?}", poll_result.unwrap_err());
                continue;
            }

            let dhcp_config = poll_result.unwrap();
            log::info!("config: {:?}", dhcp_config);

            match iface.poll(&mut sockets, instant) {
                Ok(false) => {
                    log::debug!("false!");
                    break;
                }
                Ok(true) => {
                    log::debug!("true");
                }
                Err(smoltcp::Error::Unrecognized) => {
                    log::debug!("unrecognized");
                }
                Err(err) => {
                    log::debug!("smoltcp error: {:?}", err);
                    break;
                }
            }
        }

        dhcp.next_poll(instant);

        if let Some(_timeout) = iface.poll_delay(&sockets, instant) {}
    }
}
