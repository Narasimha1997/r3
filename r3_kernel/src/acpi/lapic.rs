extern crate log;


use core::arch::asm;

use crate::acpi::madt;
use crate::mm::{io::MemoryIO, VirtualAddress};

use core::sync::atomic::{AtomicBool, Ordering};

use madt::PROCESSORS;

pub struct ProcessorID(u8);

const IA32_MSR_APIC_BASE: u32 = 0x1B;

pub fn read_msr_base_address() -> u32 {
    let base_addr_eax: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") IA32_MSR_APIC_BASE,
            out("eax") base_addr_eax,
            options(nomem, nostack)
        );
    }

    base_addr_eax & 0xfffff000
}

impl ProcessorID {
    #[inline]
    pub fn is_bsp(&self) -> bool {
        self.0 == 0
    }
}

pub enum LapicNumbers {
    LapicID = 0x20,
    LapicVersion = 0x30,
    TaskPriority = 0x80,
    ArbitrationPriority = 0x90,
    ProcessorPriority = 0xa0,
    Eoi = 0xb0,
    RemoteRead = 0xc0,
    LocalDestination = 0xd0,
    DestinationFormat = 0xe0,
    SupriousInterrupt = 0xf0,
    ISRBase = 0x100,
    TriggerModeBase = 0x180,
    InterruptRequest = 0x200,
    ErrorStatus = 0x280,
    LvtCMCI = 0x2f0,
    InterruptCommandBase = 0x300,
    LvtTimer = 0x320,
    LvtThermalSensor = 0x330,
    LvtPMCounters = 0x340,
    LvtLINT0 = 0x350,
    LvtLINT1 = 0x360,
    LvtError = 0x370,
    TimerInitialCount = 0x380,
    TimerCurrentCount = 0x390,
    TimerDivideConfig = 0x3e0,
}

/// implements IO functions used by LAPIC
pub struct LAPICRegistersIO;

impl LAPICRegistersIO {
    #[inline]
    pub fn get_base_addr() -> VirtualAddress {
        PROCESSORS.lock().lapic_address
    }

    #[inline]
    pub fn read_register(register: u64) -> u32 {
        let addr = Self::get_base_addr();
        let reader = MemoryIO::new(VirtualAddress::from_u64(addr.as_u64() + register), false);

        reader.read_u32()
    }

    #[inline]
    pub fn write_register(register: u64, value: u32) {
        let addr = Self::get_base_addr();
        let writer = MemoryIO::new(VirtualAddress::from_u64(addr.as_u64() + register), false);

        writer.write_u32(value);
    }
}

/// implements functions using which some basic LAPIC
/// operations can be carried out.
pub struct LAPICUtils;

impl LAPICUtils {
    pub fn get_processor_id() -> ProcessorID {
        let lapic_id = LAPICRegistersIO::read_register(LapicNumbers::LapicID as u64);
        ProcessorID((lapic_id >> 24) as u8)
    }

    pub fn eoi() {
        LAPICRegistersIO::write_register(LapicNumbers::Eoi as u64, 0);
    }

    pub fn setup_timer(vector: u8) {
        // unmasks the timer + configures TSC deadline mode.
        let timer_flag = (vector as u32) | 0x40000;
        LAPICRegistersIO::write_register(LapicNumbers::LvtTimer as u64, timer_flag);
    }

    #[inline]
    fn write_lapic_reg(offset: u64, data: u32) {
        let lapic_addr = LAPICRegistersIO::get_base_addr();
        let v_addr = VirtualAddress::from_u64(lapic_addr.as_u64() + offset as u64);
        let mmio_port = MemoryIO::new(v_addr, false);
        mmio_port.write_u32(data);
    }

    pub fn enable_lapic() {
        // set task priority register
        Self::write_lapic_reg(LapicNumbers::TaskPriority as u64, 0);

        // set destination format:
        Self::write_lapic_reg(LapicNumbers::DestinationFormat as u64, 0xffffffff);

        // disable LVT0 and LVT1
        Self::write_lapic_reg(LapicNumbers::LvtLINT0 as u64, 0x10000);
        Self::write_lapic_reg(LapicNumbers::LvtLINT1 as u64, 0x10000);

        // set perf monitoring
        Self::write_lapic_reg(LapicNumbers::LvtPMCounters as u64, 4 << 8);

        // set spurious interrupt
        Self::write_lapic_reg(LapicNumbers::SupriousInterrupt as u64, 0xff | 0x100);

        // disable timer
        Self::write_lapic_reg(LapicNumbers::LvtTimer as u64, 0x10000);
    }
}

static APIC_BSP_ENABLED: AtomicBool = AtomicBool::new(false);

/// init the LAPIC for base processor, i.e the processor with CPU ID 0
pub fn init_bsp_lapic() {
    if !LAPICUtils::get_processor_id().is_bsp() {
        log::warn!("BSP LAPIC init function called from a non BSP.");
        return;
    }

    // enable LAPIC
    LAPICUtils::enable_lapic();

    // set up LAPIC timer:
    LAPICUtils::setup_timer(0x50);

    log::info!("Enabled LAPIC and APIC timer for base processor.");
    APIC_BSP_ENABLED.store(true, Ordering::SeqCst);
}

pub fn bsp_apic_enabled() -> bool {
    APIC_BSP_ENABLED.load(Ordering::SeqCst)
}
