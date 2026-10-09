use super::fixture;
use std::fs;
use sysinfo::System;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Virtualization {
    IntelVtx,
    AmdV,
    Unsupported,
    /// `/proc/cpuinfo` couldn't be read at all — different from
    /// confirmed-unsupported, so the UI can say "Unknown" instead of
    /// implying the hardware lacks it.
    Unknown,
}

impl Virtualization {
    pub fn label(self) -> &'static str {
        match self {
            Virtualization::IntelVtx => "Intel VT-x",
            Virtualization::AmdV => "AMD-V",
            Virtualization::Unsupported => "Not available",
            Virtualization::Unknown => "Unknown",
        }
    }

    pub fn is_enabled(self) -> bool {
        matches!(self, Virtualization::IntelVtx | Virtualization::AmdV)
    }
}

pub struct CpuInfo {
    pub model: String,
    pub architecture: String,
    pub physical_cores: usize,
    pub logical_threads: usize,
    /// `None` when neither the sysfs cpufreq path nor sysinfo could
    /// report a figure (e.g. inside some VMs / containers).
    pub max_frequency_mhz: Option<u64>,
    pub virtualization: Virtualization,
    /// Per-core *current* frequency in MHz, in core order. Empty if
    /// sysinfo reported no cores (shouldn't happen on a real machine, but
    /// keeps the Processor tab's optional per-core grid honest about
    /// what it has).
    pub per_core_frequency_mhz: Vec<u64>,
}

pub fn collect(sys: &System) -> CpuInfo {
    let model = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown processor".to_string());

    let architecture = System::cpu_arch().unwrap_or_else(|| "Unknown".to_string());

    let logical_threads = sys.cpus().len();
    let physical_cores = sys.physical_core_count().unwrap_or(logical_threads);

    let max_frequency_mhz = max_frequency_from_sysfs().or_else(|| {
        // Fallback: sysinfo only exposes *current* frequency, but it's
        // better than nothing if cpufreq sysfs isn't present.
        sys.cpus().iter().map(|c| c.frequency()).max()
    });

    CpuInfo {
        model,
        architecture,
        physical_cores,
        logical_threads,
        max_frequency_mhz,
        virtualization: detect_virtualization(),
        per_core_frequency_mhz: sys.cpus().iter().map(|c| c.frequency()).collect(),
    }
}

/// Reads `cpuinfo_max_freq` (kHz) for cpu0 — the standard cpufreq sysfs
/// path on virtually every Linux distro. Falls back silently if the
/// kernel doesn't expose cpufreq (some VMs, some ARM boards).
fn max_frequency_from_sysfs() -> Option<u64> {
    let khz: u64 = fs::read_to_string(fixture::sys_path(
        "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq",
    ))
    .ok()?
    .trim()
    .parse()
    .ok()?;
    Some(khz / 1000)
}

/// Looks for the `vmx` (Intel VT-x) or `svm` (AMD-V) CPU flag in
/// `/proc/cpuinfo`. This reports hardware *capability*, which is what the
/// brief asks for ("if virtualization is enabled") — note that on some
/// systems the flag is present but virtualization is switched off in
/// firmware, which this can't distinguish from userspace. Cross-checking
/// `/dev/kvm` existence would narrow that gap further if needed later.
fn detect_virtualization() -> Virtualization {
    let Ok(cpuinfo) = fs::read_to_string(fixture::sys_path("/proc/cpuinfo")) else {
        return Virtualization::Unknown;
    };
    detect_virtualization_from_flags(&cpuinfo)
}

/// Pure classifier over a `/proc/cpuinfo` text, kept separate from the
/// file read so tests can drive it with canned inputs.
fn detect_virtualization_from_flags(cpuinfo: &str) -> Virtualization {
    let Some(flags_line) = cpuinfo.lines().find(|l| l.starts_with("flags")) else {
        return Virtualization::Unknown;
    };

    if flags_line.split_whitespace().any(|f| f == "vmx") {
        Virtualization::IntelVtx
    } else if flags_line.split_whitespace().any(|f| f == "svm") {
        Virtualization::AmdV
    } else {
        Virtualization::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_intel_vmx_flag() {
        let info = "processor\t: 0\nflags\t\t: fpu vmx sse2\n";
        assert_eq!(
            detect_virtualization_from_flags(info),
            Virtualization::IntelVtx
        );
        assert!(Virtualization::IntelVtx.is_enabled());
        assert_eq!(Virtualization::IntelVtx.label(), "Intel VT-x");
    }

    #[test]
    fn detects_amd_svm_flag() {
        let info = "processor\t: 0\nflags\t\t: fpu svm sse2\n";
        assert_eq!(
            detect_virtualization_from_flags(info),
            Virtualization::AmdV
        );
    }

    #[test]
    fn distinguishes_unsupported_from_unknown() {
        let no_flags = "processor\t: 0\nmodel name\t: Test CPU\n";
        assert_eq!(
            detect_virtualization_from_flags(no_flags),
            Virtualization::Unknown
        );
        let without_either = "processor\t: 0\nflags\t\t: fpu sse2\n";
        assert_eq!(
            detect_virtualization_from_flags(without_either),
            Virtualization::Unsupported
        );
        assert_eq!(Virtualization::Unsupported.label(), "Not available");
        assert!(!Virtualization::Unsupported.is_enabled());
    }
}
