pub mod dhcp;
pub mod iface;
pub mod ip_utils;
pub mod types;

extern crate log;

pub fn init_networking() {
    iface::setup_network_interface();

    if let Some(mac_address) = iface::get_formatted_mac() {

        log::info!("System MAC Address: {}", mac_address);

        types::setup_socket_set();
        dhcp::DHCPClient::init();
    }
}
