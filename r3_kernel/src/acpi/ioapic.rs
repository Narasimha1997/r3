extern crate spin;

use crate::acpi::madt;
use crate::cpu::hw_interrupts::HARDWARE_INTERRUPTS_BASE;
use crate::mm;

use core::mem;

use spin::MutexGuard;

const MAX_IOAPIC_INTERRUTPS: usize = 24;

#[derive(Debug, Copy, Clone)]
pub enum IOAPICDeliveryMode {
    Fixed = 0,
    LowPriority = 1,
    SMI = 2,
    NMI = 4,
    Init = 5,
    ExternalInt = 7,
}

pub enum IOAPICMMIOCommands {
    IOAPICRegisterID = 0,
    IOAPICRegisterVersion = 1,
    IOAPICRegisterArbID = 2,
    IOAPICRegisterReadWrite = 16,
}

#[derive(Copy, Clone)]
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
        Self::get_mmio_handle(
            ioapic_address,
            IOAPICMMIOCommands::IOAPICRegisterReadWrite as u32,
        )
        .write_u32(value);
    }

    #[inline]
    pub fn get_ioapic_version(ioapic_address: u32) -> u8 {
        Self::select_io_register(
            ioapic_address,
            IOAPICMMIOCommands::IOAPICRegisterVersion as u32,
        );

        let v_data = Self::get_mmio_handle(
            ioapic_address,
            IOAPICMMIOCommands::IOAPICRegisterReadWrite as u32,
        )
        .read_u32();
        (v_data >> 16) as u8
    }

    pub fn register_ioapic_interrupt(
        ioapics: &[madt::PerProcessorIOAPIC],
        r_entry: IOAPICRedirectEntry,
        irq_no: usize,
    ) {
        for apic in ioapics {
            if (apic.gsi_base as usize) < irq_no {
                let local_offset = irq_no - apic.gsi_base as usize;
                let max_version = Self::get_ioapic_version(apic.mmio_address);
                if local_offset <= max_version as usize {
                    // register redirect entry
                    let write_offset = (IOAPICMMIOCommands::IOAPICRegisterReadWrite as usize
                        + local_offset * 2) as u32;
                    // redirect entry is 64 bits in length
                    let redirect_flags: u64 = unsafe { mem::transmute(r_entry) };
                    // write lower 32 bits
                    Self::write_io_register(apic.mmio_address, write_offset, redirect_flags as u32);
                    // write upper 32 bits
                    Self::write_io_register(
                        apic.mmio_address,
                        write_offset + 1,
                        (redirect_flags >> 32) as u32,
                    );
                }
            }
        }
    }
}

fn register_default_interrupt(
    ioapics: &[madt::PerProcessorIOAPIC],
    base_offset: usize,
    delivery_method: IOAPICDeliveryMode,
    is_low_pin: bool,
    destination_cpu: usize,
    irq_no: usize,
    source_irq: usize,
) {
    // create a redirect entry
    let redirect_entry = IOAPICRedirectEntry::new(
        // base irq no = exceptions + hardware interrupt
        (base_offset + irq_no) as u8,
        // is not masked
        false,
        // destination cpu:
        destination_cpu as u8,
        // delivery mode
        delivery_method,
        // is this a logical CPU?
        false,
        // is this a pending interrupt?
        false,
        // has low pin parity?
        is_low_pin,
        // remote?
        false,
    );

    // register this interrupt
    IOAPICUtils::register_ioapic_interrupt(ioapics, redirect_entry, source_irq);
}

#[inline]
fn get_override_info_if_present<'a>(
    bus: usize,
    irq: usize,
    isos: &'a [madt::InterruptSourceOverride],
) -> Option<&'a madt::InterruptSourceOverride> {
    for iso_entry in isos {
        if iso_entry.bus == (bus as u8) && iso_entry.irq == (irq as u8) {
            return Some(&iso_entry);
        }
    }

    None
}

pub fn init_io_apics() {
    let mp_info: MutexGuard<madt::MultiProcessorInfo> = madt::PROCESSORS.lock();
    let base_cpu_apic_id = mp_info.cores.get(0).unwrap().apic_id;
    let all_ioapics = &mp_info.ioapics;
    let overrides = &mp_info.isos;

    for source_irq in 0..MAX_IOAPIC_INTERRUTPS {
        let mut base_irq = source_irq;
        let mut low_pin_parity = false;

        // check if the PCI device on bus 0 is connected to an overrided IRQ source
        if let Some(iso) = get_override_info_if_present(0, source_irq as usize, &overrides) {
            // base irq then becomes the gsi
            base_irq = iso.gsi as usize;
            // low pin parity?
            low_pin_parity = iso.flags & 2 != 0;
        }

        log::debug!(
            "mapping device interrupt to I/O APIC {} -> {}",
            source_irq,
            base_irq
        );

        // register this interrupt
        register_default_interrupt(
            &all_ioapics,
            HARDWARE_INTERRUPTS_BASE,
            IOAPICDeliveryMode::Fixed,
            low_pin_parity,
            base_cpu_apic_id as usize,
            base_irq,
            source_irq,
        );
    }
}
