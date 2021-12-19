extern crate bitflags;
extern crate log;

use crate::cpu::cpuid::{FlagsECX, CPU_FEATURES};
use crate::cpu::{halt_no_interrupts, io::Port};

const QEMU_SHUTDOWN: (usize, u16) = (0x604, 0x2000);
const VIRTUAL_BOX_SHUTDOWN: (usize, u16) = (0x4004, 0x3400);
const LEGACY_QEMU_SHUTDOWN: (usize, u16) = (0xb004, 0x2000);

const KBD_CONTROLLER_REBOOT: (usize, u8) = (0x64, 0xFE);

pub fn shutdown() {
    log::info!("Goodbye!");
    // TODO: Implement bare-metal shutdown using ACPI
    // is in hypervisor?
    if CPU_FEATURES.ecx.contains(FlagsECX::HYPERVISOR) {
        log::debug!("Initiating hypervisor shutdown.");
        Port::new(QEMU_SHUTDOWN.0, false).write_u16(QEMU_SHUTDOWN.1);
        Port::new(VIRTUAL_BOX_SHUTDOWN.0, false).write_u16(VIRTUAL_BOX_SHUTDOWN.1);
        Port::new(LEGACY_QEMU_SHUTDOWN.0, false).write_u16(LEGACY_QEMU_SHUTDOWN.1);
    }

    // this will render the computer useless, as CPU sleeps without interrupts.
    log::debug!("Halting CPU manually.");
    halt_no_interrupts();
}

pub fn reboot() {
    log::info!("Goodbye!");
    // send the command to ps2/keyboard controller that makes the processor reboot.
    Port::new(KBD_CONTROLLER_REBOOT.0, false).write_u8(KBD_CONTROLLER_REBOOT.1);

    log::error!("System reboot failed, shutting down!");
    shutdown();
}
