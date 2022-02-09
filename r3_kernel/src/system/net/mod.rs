pub mod dhcp;
pub mod iface;
pub mod ip_utils;
pub mod types;

pub fn init_networking() {
    iface::setup_network_interface();
    types::setup_socket_set();

    dhcp::DHCPClient::init();
    dhcp::DHCPClient::poll_dhcp_over_iface();
}
