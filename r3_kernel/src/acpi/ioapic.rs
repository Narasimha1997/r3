extern crate bit_field;
extern crate spin;

use crate::acpi::madt;
use crate::cpu::hw_interrupts::HARDWARE_INTERRUPTS_BASE;
use crate::mm;

use core::mem;
use bit_field::BitField;

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
pub struct IOAPICRedirectEntry(u64);

impl IOAPICRedirectEntry {

    pub fn new(
        ri_vector: u8,
        is_masked: bool,
        ri_destination: u8,
        delivery_method: IOAPICDeliveryMode,
        logical_dest: bool,
        pending: bool,
        low_pin: bool,
        remote: bool,
        level_trigger: bool,
    ) -> Self {
       let mut entry = 0u64;

       // first 8 bits = ri vector
       entry.set_bits(0..8, ri_vector as u64);
       // next 3 bits = delivery mode
       entry.set_bits(8..11, delivery_method as u64);
       // logical destination?
       entry.set_bit(11, logical_dest);
       // bit 12 is status
       entry.set_bit(12, pending);
       // bit 13 is pin polarity
       entry.set_bit(13, low_pin);
       // bit 14 is remote IRR
       entry.set_bit(14, remote);
       // bit 15 is trigger
       entry.set_bit(15, level_trigger);
       // bit 16 is mask
       entry.set_bit(16, is_masked);
       // bits 56-63 is the destination cpu id
       entry.set_bits(56..64, ri_destination as u64);

       log::debug!("IOAPIC entry: {:064b}", entry);

       Self(entry)
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
    level_trigger: bool,
    is_masked: bool,
) {
    // create a redirect entry
    let redirect_entry = IOAPICRedirectEntry::new(
        // base irq no = exceptions + hardware interrupt
        (base_offset + irq_no) as u8,
        // is masked?
        is_masked,
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
        // is level triggered?
        level_trigger,
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

#[inline]
fn should_mask<'a>(current_irq: u8, masks: &'a [u8]) -> bool {
    for masked_no in masks {
        if *masked_no == current_irq {
            return true;
        }
    }

    false
}

pub fn init_io_apics(masked_interrupts: &[u8]) {
    let mp_info: MutexGuard<madt::MultiProcessorInfo> = madt::PROCESSORS.lock();
    let base_cpu_apic_id = mp_info.cores.get(0).unwrap().apic_id;
    let all_ioapics = &mp_info.ioapics;
    let overrides = &mp_info.isos;

    for source_irq in 0..MAX_IOAPIC_INTERRUTPS {
        let mut base_irq = source_irq;
        let mut low_pin_parity = false;
        let mut level_trigger = false;
        let is_masked = should_mask(source_irq as u8, masked_interrupts);

        // check if the PCI device on bus 0 is connected to an overrided IRQ source
        if let Some(iso) = get_override_info_if_present(0, source_irq as usize, &overrides) {
            // base irq then becomes the gsi
            base_irq = iso.gsi as usize;
            // low pin parity?
            low_pin_parity = iso.flags & 2 != 0;
            // is it level or edge triggered?
            level_trigger = iso.flags & 8 != 0;
        }

        log::debug!(
            "mapping device interrupt to I/O APIC {} -> {}, trigger_mode={}, masked={}",
            source_irq,
            base_irq,
            if level_trigger { "level" } else { "edge" },
            is_masked
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
            level_trigger,
            is_masked,
        );
    }
}
