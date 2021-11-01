extern crate bitflags;

use lazy_static::lazy_static;

use bitflags::bitflags;
bitflags! {
    pub struct FlagsECX: u32 {
        const SSE3         = 1 << 0;
        const PCLMUL       = 1 << 1;
        const DTES64       = 1 << 2;
        const MONITOR      = 1 << 3;
        const DS_CPL       = 1 << 4;
        const VMX          = 1 << 5;
        const SMX          = 1 << 6;
        const EST          = 1 << 7;
        const TM2          = 1 << 8;
        const SSSE3        = 1 << 9;
        const CID          = 1 << 10;
        const FMA          = 1 << 12;
        const CX16         = 1 << 13;
        const ETPRD        = 1 << 14;
        const PDCM         = 1 << 15;
        const PCIDE        = 1 << 17;
        const DCA          = 1 << 18;
        const SSE4_1       = 1 << 19;
        const SSE4_2       = 1 << 20;
        const X2APIC       = 1 << 21;
        const MOVBE        = 1 << 22;
        const POPCNT       = 1 << 23;
        const TSCD         = 1 << 24;
        const AES          = 1 << 25;
        const XSAVE        = 1 << 26;
        const OSXSAVE      = 1 << 27;
        const AVX          = 1 << 28;
    }
}

bitflags! {
    pub struct FlagsEDX: u32 {
        const FPU          = 1 << 0;
        const VME          = 1 << 1;
        const DE           = 1 << 2;
        const PSE          = 1 << 3;
        const TSC          = 1 << 4;
        const MSR          = 1 << 5;
        const PAE          = 1 << 6;
        const MCE          = 1 << 7;
        const CX8          = 1 << 8;
        const APIC         = 1 << 9;
        const SEP          = 1 << 11;
        const MTRR         = 1 << 12;
        const PGE          = 1 << 13;
        const MCA          = 1 << 14;
        const CMOV         = 1 << 15;
        const PAT          = 1 << 16;
        const PSE36        = 1 << 17;
        const PSN          = 1 << 18;
        const CLF          = 1 << 19;
        const DTES         = 1 << 21;
        const ACPI         = 1 << 22;
        const MMX          = 1 << 23;
        const FXSR         = 1 << 24;
        const SSE          = 1 << 25;
        const SSE2         = 1 << 26;
        const SS           = 1 << 27;
        const HTT          = 1 << 28;
        const TM1          = 1 << 29;
        const IA64         = 1 << 30;
        const PBE          = 1 << 31;
    }
}

#[derive(Clone, Debug)]
pub struct CPUFeatures {
    pub ecx: FlagsECX,
    pub edx: FlagsEDX,
    pub max_standard_level: u32,
    pub max_extended_level: u32,
}

fn probe_cpu_features() -> CPUFeatures {
    let ecx: u32;
    let edx: u32;

    // used across this function as a exchange register.
    let mut ebx_scratch: u64;

    log::info!("Probing CPU Features with cpuid instruction.");

    unsafe {
        asm!(
            "xchg {0:r}, rbx",
            "cpuid",
            "xchg {0:r}, rbx",
            out(reg) ebx_scratch,
            inout("eax") 1 => _,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, nomem, preserves_flags)
        );
    }

    log::debug!(
        "cpuid register ecx=0x{:x}, edx=0x{:x}, ebx={:x}",
        ecx,
        edx,
        ebx_scratch
    );

    // probe CPU level:
    let (max_standard_level, max_extended_level): (u32, u32);

    unsafe {
        // load standard levels:
        asm!(
            "xchg {0:r}, rbx",
            "cpuid",
            "xchg {0:r}, rbx",
            out(reg) ebx_scratch,
            inout("eax") 0 => max_standard_level,
            out("rdx") _,
            out("rcx") _,
            options(nostack, nomem, preserves_flags)
        );

        log::debug!(
            "CPUID max_standard_level={:x}, ebx={:x}.",
            max_standard_level,
            ebx_scratch
        );

        // load extended levels:
        asm!(
            "xchg {0:r}, rbx",
            "cpuid",
            "xchg {0:r}, rbx",
            out(reg) ebx_scratch,
            inout("eax") 0x8000_0000u32 => max_extended_level,
            out("rdx") _,
            out("rcx") _,
            options(nostack, nomem, preserves_flags)
        );

        log::debug!(
            "CPUID max_extended_level={:x}, ebx={:x}.",
            max_extended_level,
            ebx_scratch
        );
    }

    CPUFeatures {
        ecx: FlagsECX::from_bits_truncate(ecx),
        edx: FlagsEDX::from_bits_truncate(edx),
        max_extended_level,
        max_standard_level,
    }
}

lazy_static! {
    static ref CPU_FEATURES: CPUFeatures = probe_cpu_features();
}

pub fn has_feature(flag: FlagsECX) -> bool {
    CPU_FEATURES.ecx.contains(flag)
}

pub fn has_extended_feature(flag: FlagsEDX) -> bool {
    CPU_FEATURES.edx.contains(flag)
}

pub fn warn_levels() {
    assert!(
        CPU_FEATURES.max_standard_level < 3,
        "Expected CPU standard level >= 3, got 0x{:x}.",
        CPU_FEATURES.max_standard_level
    );

    assert!(
        CPU_FEATURES.max_extended_level >= 0x8000_0007,
        "Expected CPU extended level >= 0x80000007, got 0x{:x}.",
        CPU_FEATURES.max_extended_level
    );

    log::info!("CPU level checks passed.");
}

pub fn assert_feature(flag: FlagsECX) {
    assert!(
        has_feature(flag),
        "CPU does not support critical standard feature {:?}",
        flag
    );
}

pub fn assert_extended_feature(flag: FlagsEDX) {
    assert!(
        has_extended_feature(flag),
        "CPU does not support critical extended feature {:?}",
        flag
    );
}

pub fn display_features() {
    log::info!("Feature Register ecx={:?}", CPU_FEATURES.ecx);
    log::info!("Feature Register edx={:?}", CPU_FEATURES.edx);
    log::info!("Max standard level 0x{:x}", CPU_FEATURES.max_standard_level);
    log::info!("Max extended level 0x{:x}", CPU_FEATURES.max_extended_level);
}
