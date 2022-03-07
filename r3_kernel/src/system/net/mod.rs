pub mod dhcp;
pub mod iface;
pub mod ip_utils;
pub mod types;
pub mod process;
pub mod udp;

extern crate log;

pub fn init_networking() {
    iface::setup_network_interface();
    types::setup_socket_set();

    let mac_address_opt = iface::get_formatted_mac();

    if let Some(mac_address) = mac_address_opt {
        log::info!("Network interface MAC: {}", mac_address);
        // 1. Initialize DHCP and probe for dynamic IP
        dhcp::DHCPClient::init();

        // enable polling mode for initial configuration:
        iface::set_polling_mode();

        if let Ok(_) = dhcp::DHCPClient::configure_iface_via_dhcp() {
            log::info!("DHCP configuration complete");
        } else {
            log::info!("DHCP configuration failed, falling back to static IP");
            iface::switch_to_static_ip();
        }

        // switch back to interrupt mode
        iface::set_interrupt_mode();
    } else {
        log::error!("network interface did not provide any MAC address, skipping network init");
    }
}
