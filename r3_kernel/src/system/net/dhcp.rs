extern crate alloc;
extern crate lazy_static;
extern crate log;
extern crate smoltcp;
extern crate spin;

use alloc::vec;
use core::time::Duration;
use lazy_static::lazy_static;
use smoltcp::dhcp::{Dhcpv4Client, Dhcpv4Config};
use smoltcp::socket::{RawPacketMetadata, RawSocketBuffer, SocketSet};
use smoltcp::time::Instant;
use smoltcp::wire::IpCidr;
use spin::{Mutex, MutexGuard};

use crate::system::net::iface;
use crate::system::net::types::SOCKETS_SET;
use crate::system::timer::{wait_ns, PosixTimeval};

const DHCP_BUFFER_SIZE: usize = 2048;

lazy_static! {
    pub static ref DHCP_CLIENT: Mutex<Option<Dhcpv4Client>> = Mutex::new(None);
}

pub type LockedDHCPClient = MutexGuard<'static, Option<Dhcpv4Client>>;

pub enum DHCPError {
    PollingError,
    UnrecognizedPacket,
    NoInterface,
    NoDHCPClient,
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

    fn poll_dhcp_over_iface(
        iface_lock: &mut iface::LockedEthernetInterface,
        dhcp_lock: &mut LockedDHCPClient,
    ) -> Result<Dhcpv4Config, DHCPError> {
        if iface_lock.as_ref().is_none() {
            log::error!("cannot poll DHCP over empty interface");
            return Err(DHCPError::NoInterface);
        }
        let mut iface = iface_lock.as_mut().unwrap();
        if dhcp_lock.is_none() {
            log::error!("cannot poll over empty DHCP client");
            return Err(DHCPError::NoDHCPClient);
        }
        let dhcp = dhcp_lock.as_mut().unwrap();

        let ts = PosixTimeval::from_ticks().mills();
        let instant = Instant::from_millis(ts as i64);

        let mut sockets_lock = SOCKETS_SET.lock();
        let mut sockets = sockets_lock.as_mut().unwrap();

        loop {
            match iface.poll(&mut sockets, instant) {
                Ok(false) => {}
                Ok(true) => {}
                Err(smoltcp::Error::Unrecognized) => {
                    log::debug!("unrecognized");
                    return Err(DHCPError::UnrecognizedPacket);
                }

                Err(err) => {
                    log::debug!("smoltcp error: {:?}", err);
                    return Err(DHCPError::PollingError);
                }
            }

            let poll_result = dhcp.poll(&mut iface, &mut sockets, instant);
            if poll_result.is_err() {
                log::error!("DHCP poll error: {:?}", poll_result.unwrap_err());
                return Err(DHCPError::PollingError);
            }

            let dhcp_config_opt = poll_result.unwrap();
            if let Some(dhcp_config) = dhcp_config_opt {
                return Ok(dhcp_config);
            }

            if let Some(duration) = iface.poll_delay(&sockets, instant) {
                // wait for sometime
                let d: Duration = duration.into();
                wait_ns(d.as_nanos() as u64);
            }
        }
    }

    /// this function polls using DHCP client and returns the DHCPConfig
    /// can be used later in the interrupt based handler for dynamic DHCP address lease changes.
    pub fn check_dhcp_packet(
        iface_lock: &mut iface::LockedEthernetInterface,
        dhcp_lock: &mut LockedDHCPClient,
        instant: Instant,
        sockets: &mut SocketSet,
    ) -> Result<Option<Dhcpv4Config>, DHCPError> {
        if iface_lock.as_ref().is_none() {
            log::error!("cannot poll DHCP over empty interface");
            return Err(DHCPError::NoInterface);
        }
        let mut iface = iface_lock.as_mut().unwrap();
        if dhcp_lock.is_none() {
            log::error!("cannot poll over empty DHCP client");
            return Err(DHCPError::NoDHCPClient);
        }
        let dhcp = dhcp_lock.as_mut().unwrap();

        // poll and return:
        let poll_result = dhcp.poll(&mut iface, sockets, instant);
        if poll_result.is_err() {
            log::error!("DHCP poll error: {:?}", poll_result.unwrap_err());
            return Err(DHCPError::PollingError);
        }

        let dhcp_config_opt = poll_result.unwrap();
        if let Some(dhcp_config) = dhcp_config_opt {
            return Ok(Some(dhcp_config));
        }

        return Ok(None);
    }

    pub fn dhcp_next_poll(dhcp_lock: &mut LockedDHCPClient, instant: Instant) {
        if dhcp_lock.is_none() {
            log::error!("cannot poll over empty DHCP client");
            return;
        }

        let dhcp = dhcp_lock.as_mut().unwrap();
        dhcp.next_poll(instant);
    }

    /// modify the interface routes with new DHCP configuration
    pub fn update_config(
        config: Dhcpv4Config,
        iface_lock: &mut iface::LockedEthernetInterface,
    ) -> Result<(), DHCPError> {
        if iface_lock.as_ref().is_none() {
            log::error!("cannot poll DHCP over empty interface");
            return Err(DHCPError::NoInterface);
        }

        let iface = iface_lock.as_mut().unwrap();

        // 1. Get the new CIDR and replace the current one:
        if let Some(cidr_addr) = config.address {
            log::info!("Assigning new IP address {}", cidr_addr);
            iface.update_ip_addrs(|current_addresses| {
                for addr in current_addresses.iter_mut() {
                    *addr = IpCidr::Ipv4(cidr_addr);
                }
            });
        }

        // 2. Update the default gateway:
        if let Some(router) = config.router {
            log::info!("Assigning new gateway route {}", router);
            iface
                .routes_mut()
                .add_default_ipv4_route(router)
                .expect("failed to add a new ipv4 gateway route.");
        }

        // 3. TODO: Do something with DNS address
        Ok(())
    }

    /// used for DHCP initial probing while bootup
    pub fn configure_iface_via_dhcp() -> Result<(), DHCPError> {
        let mut iface_lock = iface::get_virtual_interface().lock();
        let mut dhcp_lock = DHCP_CLIENT.lock();

        let poll_result = Self::poll_dhcp_over_iface(&mut iface_lock, &mut dhcp_lock);
        if let Ok(dhcp_config) = poll_result {
            return Self::update_config(dhcp_config, &mut iface_lock);
        } else {
            return Err(poll_result.unwrap_err());
        }
    }
}
