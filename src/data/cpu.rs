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
    /// Unique caches by (level, size, shared set), lowest level first.
    /// Empty where the kernel doesn't expose the cache tree (some VMs).
    pub cache: Vec<CacheInfo>,
}

/// One cache level as reported by sysfs, e.g. L3 at 8 MB shared across
/// all cores of the package.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CacheInfo {
    pub level: u8,
    pub size_bytes: u64,
}

impl CacheInfo {
    /// "L3 8 MB" for the Processor facts row.
    pub fn label(self) -> String {
        format!("L{} {}", self.level, super::format_bytes(self.size_bytes))
    }
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
        cache: detect_cache(),
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

/// Cache tree from `/sys/devices/system/cpu/cpu*/cache/index*`. Scans
/// every CPU but deduplicates on (level, size): a package-wide L3 is
/// identical under each core, and per-core L1/L2 repeat at the same size,
/// so each distinct level+size appears once. Routed through the fixture
/// root in tests.
fn detect_cache() -> Vec<CacheInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut cache = Vec::new();
    for cpu in 0..256 {
        let mut any = false;
        for index in 0..16 {
            let base = fixture::sys_path(&format!(
                "/sys/devices/system/cpu/cpu{cpu}/cache/index{index}"
            ));
            let (Some(level), Some(size)) = (
                read_small(&base.join("level")),
                read_small(&base.join("size")),
            ) else {
                continue;
            };
            any = true;
            let level: u8 = match level.trim().parse() {
                Ok(level) => level,
                Err(_) => continue,
            };
            let Some(size_bytes) = parse_cache_size(size.trim()) else {
                continue;
            };
            if seen.insert((level, size_bytes)) {
                cache.push(CacheInfo { level, size_bytes });
            }
        }
        if !any {
            break;
        }
    }
    cache.sort_by_key(|entry| entry.level);
    cache
}

fn read_small(path: &std::path::Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// Pure parser for sysfs cache sizes (`1024K`, `16M`, `256M`).
fn parse_cache_size(size: &str) -> Option<u64> {
    let (digits, factor) = match size.strip_suffix(['K', 'M', 'G']) {
        Some(rest) if size.ends_with('K') => (rest, 1024),
        Some(rest) if size.ends_with('M') => (rest, 1024 * 1024),
        Some(rest) => (rest, 1024 * 1024 * 1024),
        None => (size, 1),
    };
    digits.parse::<u64>().ok().map(|value| value * factor)
}
/// Looks for the `vmx` (Intel VT-x) or `svm` (AMD-V) CPU flag in
/// `/proc/cpuinfo`. Reports hardware *capability*; on some systems the
/// flag is present but virtualization is switched off in firmware, which
/// this can't distinguish from userspace.
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
    fn parses_sysfs_cache_sizes() {
        assert_eq!(parse_cache_size("768K"), Some(768 * 1024));
        assert_eq!(parse_cache_size("8192K"), Some(8 * 1024 * 1024));
        assert_eq!(parse_cache_size("16M"), Some(16 * 1024 * 1024));
        assert_eq!(parse_cache_size("nonsense"), None);
        assert_eq!(
            CacheInfo {
                level: 3,
                size_bytes: 8 * 1024 * 1024
            }
            .label(),
            "L3 8.0 MB"
        );
    }

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
