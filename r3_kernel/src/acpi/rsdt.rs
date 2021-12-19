extern crate log;
extern crate spin;

use crate::boot_proto::BootProtocol;
use crate::mm::{p_to_v, PhysicalAddress, VirtualAddress};

use core::mem;
use core::str;
use lazy_static::lazy_static;
use spin::Mutex;

const RSDT_SIG: &str = "RSD PTR ";

const MAX_ACPI_TABLES: usize = 48;

pub struct Acpi {
    pub tables: [VirtualAddress; MAX_ACPI_TABLES],
    pub n_entries: usize,
    pub supports_2x: bool,
}

#[derive(Debug)]
/// Enum type that says whether the table is RSDT or XSDT
pub enum AcpiRootTableKind {
    RSDT(u32),
    XSDT(u64),
}

#[derive(Debug)]
pub enum AcpiRootTableError {
    NotFound,
    InvalidSignature,
    InvalidChecksum,
    InvalidChecksum2x,
}

#[repr(C, packed)]
/// The struct representing RSDP descriptor
pub struct RSDPDescriptor {
    signature: [u8; 8],
    checksum_byte: u8,
    oem: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
/// Extends RSDPDescritor with extra fields as per acpi2x specification.
pub struct RSDPDescriptor2x {
    legacy_descriptor: RSDPDescriptor,
    length: u32,
    xsdt_address: u64,
    checksum_2x: u8,
    reserved_bytes: [u8; 3],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SDTHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_rev: u32,
    pub creator_id: u32,
    pub creator_rev: u32,
}

#[inline]
pub fn is_extended_rsdp(rsdp: &RSDPDescriptor) -> bool {
    rsdp.revision == 2
}

impl AcpiRootTableKind {
    pub fn parse_from_bootinfo() -> Result<AcpiRootTableKind, AcpiRootTableError> {
        let boot_info = BootProtocol::get_boot_proto();
        let rsdp_opt = boot_info.unwrap().rsdp_addr;

        if rsdp_opt.into_option().is_none() {
            return Err(AcpiRootTableError::NotFound);
        }

        let rsdp_addr = p_to_v(PhysicalAddress::from_u64(rsdp_opt.into_option().unwrap()));

        // ACPI 1.0 RSDT is 20 bytes
        let bytes_slice: &[u8; 20] = unsafe { &*rsdp_addr.get_ptr() };
        let rsdp_struct: &RSDPDescriptor = unsafe { &*rsdp_addr.get_ptr() };
        // verify checksum and signature:
        unsafe {
            if str::from_utf8_unchecked(&rsdp_struct.signature) != RSDT_SIG {
                log::error!("Invalid rsdp signature, expected={}", RSDT_SIG);
                return Err(AcpiRootTableError::InvalidSignature);
            }
        }

        // verify checksum:
        let legacy_checksum: usize = bytes_slice.iter().map(|val| *val as usize).sum();
        if legacy_checksum & 0xff != 0 {
            // non zero ending checksum:
            log::error!("Invalid legacy rsdt checksum");
            return Err(AcpiRootTableError::InvalidChecksum);
        }

        // check if the table has 2.0 version support.
        if is_extended_rsdp(&rsdp_struct) {
            // supports version 2.0
            let ext_bytes_slice: &[u8; 36] = unsafe { &*rsdp_addr.get_ptr() };
            let ext_rsdp_struct: &RSDPDescriptor2x = unsafe { &*rsdp_addr.get_ptr() };

            // verify extended checksum:
            let ext_checksum: usize = ext_bytes_slice.iter().map(|val| *val as usize).sum();
            if ext_checksum & 0xff != 0 {
                log::error!("Invalid ACPI 2.0 checksum.");
                return Err(AcpiRootTableError::InvalidChecksum2x);
            }

            return Ok(AcpiRootTableKind::XSDT(ext_rsdp_struct.xsdt_address));
        }

        return Ok(AcpiRootTableKind::RSDT(rsdp_struct.rsdt_address));
    }
}

pub fn init_acpi() -> Option<Acpi> {
    let root_table_result = AcpiRootTableKind::parse_from_bootinfo();
    if root_table_result.is_err() {
        log::error!("ACPI Init Error: {:?}", root_table_result.unwrap_err());
        return None;
    }

    log::info!("Initializing ACPI tables.");

    // match root table type:
    let root_table = root_table_result.unwrap();
    let (head_addr, ptr_size, supports_2x) = match root_table {
        AcpiRootTableKind::RSDT(addr) => (
            PhysicalAddress::from_u64(addr as u64),
            mem::size_of::<u32>(),
            false,
        ),
        AcpiRootTableKind::XSDT(addr) => {
            (PhysicalAddress::from_u64(addr), mem::size_of::<u64>(), true)
        }
    };

    log::info!("acpi_2x support={}", supports_2x);

    let head_v_addr = p_to_v(head_addr);
    let root_header: &SDTHeader = unsafe { &*head_v_addr.get_ptr() };
    unsafe {
        if !supports_2x {
            if str::from_utf8_unchecked(&root_header.signature) != "RSDT" {
                log::error!("Invalid root table header, expected RSDT.");
                return None;
            }
        } else {
            if str::from_utf8_unchecked(&root_header.signature) != "XSDT" {
                log::error!("Invalid root table header, expected XSDT.");
                return None;
            }
        }
    }

    let mut n_tables = (root_header.length as usize - mem::size_of::<SDTHeader>()) / ptr_size;
    if n_tables > MAX_ACPI_TABLES {
        log::warn!(
            "Provided ACPI tables ({}) exceeds maximum allowed tables ({}).",
            n_tables,
            MAX_ACPI_TABLES
        );
        n_tables = MAX_ACPI_TABLES;
    }

    let mut acpi_tables = [VirtualAddress::from_u64(0); MAX_ACPI_TABLES];

    // iterate over the tables:
    for idx in 0..n_tables {
        let address = p_to_v(PhysicalAddress::from_u64(
            head_addr.as_u64() + (mem::size_of::<SDTHeader>() + idx * ptr_size) as u64,
        ));

        let table_address = match root_table {
            AcpiRootTableKind::RSDT(_) => {
                let ptr: u32 = unsafe { *address.get_ptr() };
                p_to_v(PhysicalAddress::from_u64(ptr as u64))
            }
            AcpiRootTableKind::XSDT(_) => {
                let ptr: u64 = unsafe { *address.get_ptr() };
                p_to_v(PhysicalAddress::from_u64(ptr))
            }
        };

        acpi_tables[idx] = table_address;
    }

    Some(Acpi {
        tables: acpi_tables,
        n_entries: n_tables,
        supports_2x,
    })
}

impl Acpi {
    fn has_signature(&self, idx: usize, signature: &str) -> bool {
        unsafe {
            let sdt_header: &SDTHeader = &*self.tables[idx].get_ptr();
            let st = str::from_utf8_unchecked(&sdt_header.signature);
            st == signature
        }
    }

    pub fn list_tables(&self) {
        unsafe {
            log::info!("Following are the ACPI tables: ");
            for idx in 0..self.n_entries {
                let sdt_header: &SDTHeader = &*self.tables[idx].get_ptr();
                let st = str::from_utf8_unchecked(&sdt_header.signature);
                log::info!("{} {}", idx + 1, st);
            }
        }
    }

    pub fn get_table(&self, signature: &str) -> Option<VirtualAddress> {
        for idx in 0..self.n_entries {
            if self.has_signature(idx, signature) {
                return Some(self.tables[idx]);
            }
        }

        None
    }
}

lazy_static! {
    pub static ref ACPI: Mutex<Option<Acpi>> = Mutex::new(init_acpi());
}

pub fn setup_acpi() {
    let acpi_opt = ACPI.lock();
    if acpi_opt.is_some() {
        let acpi = acpi_opt.as_ref().unwrap();
        log::info!(
            "ACPI initialized, n_entries={}, supports_acpi_2={}",
            acpi.n_entries,
            acpi.supports_2x
        );

        acpi.list_tables();
    }
}
