extern crate log;
extern crate smoltcp;

use crate::system::net;
use crate::system::timer;

use smoltcp::time::Instant;
use smoltcp::Error;

use net::dhcp::{DHCPClient, DHCP_CLIENT};
use net::iface::ETHERNET_INTERFACE;
use net::types::SOCKETS_SET;

pub fn process_network_packet_event() {

    let mut iface_lock = ETHERNET_INTERFACE.lock();

    let mut dhcp_lock = DHCP_CLIENT.lock();

    let mut sockets_lock = SOCKETS_SET.lock();

    let mut sockets = sockets_lock.as_mut().unwrap();

    let ts = timer::PosixTimeval::from_ticks().mills();
    let instant = Instant::from_millis(ts as i64);

    loop {
        // check if it is DHCP packet
        let dhcp_result =
            DHCPClient::check_dhcp_packet(&mut iface_lock, &mut dhcp_lock, instant, &mut sockets);

        if dhcp_result.is_err() {
            log::debug!("DHCP Error: {:?}", dhcp_result.unwrap_err());
            break;
        }

        if let Some(dhcp_config) = dhcp_result.unwrap() {
            // we got a new DHCP config, update it
            let update_result = DHCPClient::update_config(dhcp_config, &mut iface_lock);
            if update_result.is_err() {
                log::debug!("DHCP error: {:?}", update_result.unwrap_err());
                break;
            }
        }

        // poll over the iface
        let poll_result = iface_lock.as_mut().unwrap().poll(&mut sockets, instant);
        match poll_result {
            Ok(false) => break,
            Ok(true) => {}
            Err(Error::Unrecognized) => {}
            Err(err) => {
                log::debug!("network error: {:?}", err);
                break;
            }
        }

    }

    DHCPClient::dhcp_next_poll(&mut dhcp_lock, instant);

    if let Some(_ts) = iface_lock.as_mut().unwrap().poll_delay(&sockets, instant) {}
}
