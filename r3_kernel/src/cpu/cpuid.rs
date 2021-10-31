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
}

fn probe_cpu_features() -> CPUFeatures {
    let ecx: u32;
    let edx: u32;

    let ebx_scratch: u64;

    log::info!("Probing CPU Features with cpuid instruction.");

    unsafe {
        asm!(
            "movq {0:r}, rbx",
            "cpuid",
            "xchgq {0:r}, rbx",
            out(reg) ebx_scratch,
            inout("eax") 1 => _,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, nomem)
        );
    }

    log::debug!("cpuid register ecx=0x{:x}, edx=0x{:x}", ecx, edx);

    CPUFeatures {
        ecx: FlagsECX::from_bits_truncate(ecx),
        edx: FlagsEDX::from_bits_truncate(edx),
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

pub fn assert_feature(flag: FlagsECX) {
    assert_eq!(has_feature(flag), true);
}

pub fn assert_extended_feature(flag: FlagsEDX) {
    assert_eq!(has_extended_feature(flag), true);
}

pub fn display_features() {
    log::info!("Feature Register ecx={:?}", CPU_FEATURES.ecx);
    log::info!("Feature Register edx={:?}", CPU_FEATURES.edx);
}