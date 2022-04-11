
/// Core address type which represents a 64-bit unsigned integer
pub type Address = u64;

pub enum Stdio {
    Stdin = 0,
    Stdout = 1
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct UTSName {
    pub sys_name: [u8; 65],
    pub node_name: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domain: [u8; 65],
}

impl UTSName {
    pub fn empty() -> Self {
        Self {
            sys_name: [0; 65],
            node_name: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domain: [0; 65],
        }
    }
}