extern crate smoltcp;

use smoltcp::wire::Ipv4Address;

pub fn get_ipv4_from_string(ip_string: &str) -> Option<Ipv4Address> {
    let mut parsed_octet_bytes: [u8; 4] = [0; 4];
    let octet_iter = ip_string.splitn(4, '.');

    for (idx, octet) in octet_iter.enumerate() {
        if let Ok(parsed_octet) = octet.parse::<u8>() {
            parsed_octet_bytes[idx] = parsed_octet;
        } else {
            return None;
        }
    }

    Some(Ipv4Address::from_bytes(&parsed_octet_bytes))
}

pub fn get_ipv4_with_prefix_from_string(ip_string: &str) -> Option<(Ipv4Address, usize)> {
    let mut ip_iter = ip_string.splitn(2, '/');
    let ip_addr = ip_iter.next()?;

    if let Some(ipv4) = get_ipv4_from_string(&ip_addr) {
        // parse prefix
        let prefix = ip_iter.next()?;
        if let Ok(parsed_prefix) = prefix.parse::<u8>() {
            return Some((ipv4, parsed_prefix as usize));
        }
    }

    return None;
}
