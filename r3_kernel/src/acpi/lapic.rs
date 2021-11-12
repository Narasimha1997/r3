extern crate log;

use crate::acpi::madt;
use crate::cpu::pic::disable_legacy_interrupts;
use crate::mm::{io::MemoryIO, VirtualAddress};

use core::sync::atomic::{AtomicBool, Ordering};

use madt::PROCESSORS;

pub struct ProcessorID(u8);

impl ProcessorID {
    #[inline]
    pub fn is_bsp(&self) -> bool {
        self.0 == 0
    }
}

/// offset of spurious vector register
const SUPRIOUS_VECTOR_OFFSET: u8 = 0xf0;

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

    pub fn enable_lapic() {
        // disable legacy interrupts
        disable_legacy_interrupts();
        let lapic_addr = LAPICRegistersIO::get_base_addr();

        let suprious_vec_addr =
            VirtualAddress::from_u64(lapic_addr.as_u64() + SUPRIOUS_VECTOR_OFFSET as u64);

        let mmio_port = MemoryIO::new(suprious_vec_addr, false);
        let current_value = mmio_port.read_u32();

        // https://wiki.osdev.org/APIC#Spurious_Interrupt_Vector_Registers
        mmio_port.write_u32(current_value | 0xff | 0x100);
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
    LAPICUtils::setup_timer(0x30);

    log::info!("Enabled LAPIC and APIC timer for base processor.");
    APIC_BSP_ENABLED.store(true, Ordering::SeqCst);
}

pub fn bsp_apic_enabled() -> bool {
    APIC_BSP_ENABLED.load(Ordering::SeqCst)
}
