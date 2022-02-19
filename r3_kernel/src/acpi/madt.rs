extern crate alloc;
extern crate log;
extern crate spin;

use alloc::vec::Vec;
use core::mem;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::acpi::rsdt;
use crate::mm;

use mm::{p_to_v, PhysicalAddress, VirtualAddress};
use rsdt::{SDTHeader, ACPI};

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct PerProcessorLAPIC {
    pub id: u8,
    pub apic_id: u8,
    flags: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct PerProcessorIOAPIC {
    pub ioapic_id: u8,
    reserved: u8,
    pub mmio_address: u32,
    pub gsi: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct InterruptSourceOverride {
    pub bus: u8,
    pub irq: u8,
    pub gsi: u32,
    pub flags: u16,
}


#[derive(Debug)]
pub struct MultiProcessorInfo {
    pub cores: Vec<PerProcessorLAPIC>,
    pub ioapics: Vec<PerProcessorIOAPIC>,
    pub isos: Vec<InterruptSourceOverride>,
    pub lapic_address: VirtualAddress,
}

#[derive(Debug)]
pub enum MADTError {
    NoTable,
    InvalidTableData,
}

pub struct MADT;

// some helper LAPIC structs
#[repr(C, packed)]
struct LAPICRootHeader {
    pub header: SDTHeader,
    pub lapic_phy_addr: u32,
    pub lapic_flags: u32,
}

#[repr(C, packed)]
struct LAPICEntry {
    pub entry_type: u8,
    pub entry_size: u8,
}

impl MADT {
    pub fn probe_cpu_cores() -> Result<MultiProcessorInfo, MADTError> {
        let acpi_lock = ACPI.lock();
        if acpi_lock.is_none() {
            log::error!("ACPI not initialized");
            return Err(MADTError::NoTable);
        }

        let madt_entry_opt = acpi_lock.as_ref().unwrap().get_table("APIC");
        if madt_entry_opt.is_none() {
            log::error!("APIC MADT not found");
            return Err(MADTError::NoTable);
        }

        let madt_address = madt_entry_opt.unwrap();

        let lapic_root: &LAPICRootHeader = unsafe { &*madt_address.get_ptr() };
        assert_eq!(lapic_root.header.length > 8, true);

        let mut cores: Vec<PerProcessorLAPIC> = Vec::new();
        let mut ioapics: Vec<PerProcessorIOAPIC> = Vec::new();
        let mut isos: Vec<InterruptSourceOverride> = Vec::new();

        let table_end = madt_address.as_u64() + lapic_root.header.length as u64;

        let mut entries_start = madt_address.as_u64() + mem::size_of::<LAPICRootHeader>() as u64;

        log::debug!("APIC Tables size: {}", table_end - entries_start);

        // iterate over the entries:
        while entries_start < table_end {
            let lapic_entry_addr = VirtualAddress::from_u64(entries_start);
            let lapic_entry: &LAPICEntry = unsafe { &*lapic_entry_addr.get_ptr() };

            if lapic_entry.entry_type == 0 {
                // processor apic type entry:
                let body_addr = entries_start + mem::size_of::<LAPICEntry>() as u64;
                let proc_entry: PerProcessorLAPIC = unsafe { *(body_addr as *const _) };
                log::debug!("found processor entry: {}", proc_entry.id);
                cores.push(proc_entry);
            }

            if lapic_entry.entry_type == 1 {
                // IOAPIC type entry:
                let body_addr = entries_start + mem::size_of::<LAPICEntry>() as u64;
                let ioapic_entry: PerProcessorIOAPIC = unsafe { *(body_addr as *const _) };
                log::debug!("found ioapic entry: {}", ioapic_entry.ioapic_id);
                ioapics.push(ioapic_entry);
            }

            if lapic_entry.entry_type == 2 {
                // IRQ overrides information:
                let body_addr = entries_start + mem::size_of::<LAPICEntry>() as u64;
                let iso_entry: InterruptSourceOverride = unsafe { *(body_addr as *const _) };
                log::debug!(
                    "found iso entry: irq={}, bus={}",
                    iso_entry.irq,
                    iso_entry.bus,
                );
                isos.push(iso_entry);
            }

            entries_start = entries_start + lapic_entry.entry_size as u64;
        }

        let lapic_address = p_to_v(PhysicalAddress::from_u64(lapic_root.lapic_phy_addr as u64));
        Ok(MultiProcessorInfo {
            cores,
            ioapics,
            isos,
            lapic_address,
        })
    }
}

pub fn probe_cpus() -> MultiProcessorInfo {
    let probe_res = MADT::probe_cpu_cores();
    if probe_res.is_err() {
        panic!("Failed to detect CPUs. {:?}", probe_res.unwrap_err());
    }

    probe_res.unwrap()
}

lazy_static! {
    pub static ref PROCESSORS: Mutex<MultiProcessorInfo> = Mutex::new(probe_cpus());
}

pub fn setup_madt() {
    let proc_lock = PROCESSORS.lock();
    log::info!(
        "Number of CPU cores: {}, Local APIC Address: 0x{:x}",
        proc_lock.cores.len(),
        proc_lock.lapic_address.as_u64()
    );

    for proc in &proc_lock.cores {
        log::info!("CPU-{} - {}", proc.id, proc.apic_id);
    }
}
