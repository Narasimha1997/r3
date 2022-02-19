use crate::mm;

const IOAPIC_MMIO_WRITE_OFFSET: u32 = 0x10;

#[derive(Debug, Copy, Clone)]
pub enum IOAPICDeliveryMode {
    Fixed = 0,
    LowPriority = 1,
    SMI = 2,
    NMI = 4,
    Init = 5,
    ExternalInt = 7,
}

pub enum IOAPICGeneralInfo {
    IOAPICRegisterID = 0,
    IOAPICRegisterVersion = 1,
    IOAPICRegisterArbID = 2,
}

#[repr(C, packed)]
pub struct IOAPICRedirectEntry {
    ri_vector: u8,
    ri_flags: u8,
    is_masked: bool,
    reserved: [u8; 4],
    ri_destination: u8,
}

impl IOAPICRedirectEntry {
    #[inline]
    fn prepare_ri_flags(
        delivery_method: IOAPICDeliveryMode,
        logical_dest: bool,
        pending: bool,
        low_pin: bool,
        remote: bool,
    ) -> u8 {
        (delivery_method as u8)
            | ((logical_dest as u8) << 4)
            | ((pending as u8) << 5)
            | ((low_pin as u8) << 6)
            | ((remote as u8) << 7)
    }

    pub fn new(
        ri_vector: u8,
        is_masked: bool,
        ri_destination: u8,
        delivery_method: IOAPICDeliveryMode,
        logical_dest: bool,
        pending: bool,
        low_pin: bool,
        remote: bool,
    ) -> Self {
        let reserved: [u8; 4] = [0; 4];
        let ri_flags =
            Self::prepare_ri_flags(delivery_method, logical_dest, pending, low_pin, remote);
        Self {
            ri_vector,
            ri_flags,
            is_masked,
            reserved,
            ri_destination,
        }
    }
}

pub struct IOAPICUtils;

impl IOAPICUtils {
    #[inline]
    pub fn get_ioapic_vaddr(ioapic_address: u32, offset: u32) -> mm::VirtualAddress {
        let phy_addr = mm::PhysicalAddress::from_u64((ioapic_address + offset) as u64);
        mm::p_to_v(phy_addr)
    }

    #[inline]
    pub fn get_mmio_handle(ioapic_address: u32, offset: u32) -> mm::io::MemoryIO {
        let virt_addr = Self::get_ioapic_vaddr(ioapic_address, offset);
        mm::io::MemoryIO::new(virt_addr, false)
    }

    #[inline]
    pub fn select_io_register(ioapic_address: u32, offset: u32) {
        let mmio = Self::get_mmio_handle(ioapic_address, 0);
        mmio.write_u32(offset);
    }

    #[inline]
    pub fn write_io_register(ioapic_address: u32, offset: u32, value: u32) {
        Self::select_io_register(ioapic_address, offset);
        Self::get_mmio_handle(ioapic_address, IOAPIC_MMIO_WRITE_OFFSET).write_u32(
            value
        );
    }
}
