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
    /// From cpufreq's `cpuinfo_max_freq`; `None` where cpufreq isn't
    /// exposed (e.g. inside some VMs / containers).
    pub max_frequency_mhz: Option<u64>,
    /// Lowest frequency the CPU can run at, from cpufreq. `None` where
    /// cpufreq isn't exposed.
    pub min_frequency_mhz: Option<u64>,
    /// Driver, governor and boost state from the same cpufreq policy.
    pub scaling: FrequencyScaling,
    pub virtualization: Virtualization,
    /// Unique caches by (level, size, shared set), lowest level first.
    /// Empty where the kernel doesn't expose the cache tree (some VMs).
    pub cache: Vec<CacheInfo>,
}

/// How the kernel scales CPU frequency, from cpu0's cpufreq policy.
/// Each field is `None` where its sysfs file is missing (VMs,
/// containers, drivers that don't expose it), so the Processor tab can
/// omit the row instead of guessing.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FrequencyScaling {
    /// e.g. `amd-pstate-epp`, `intel_pstate`, `acpi-cpufreq`.
    pub driver: Option<String>,
    /// e.g. `powersave`, `performance`, `schedutil`.
    pub governor: Option<String>,
    /// EPP hint, e.g. `balance_performance`; only on EPP-capable drivers.
    pub energy_preference: Option<String>,
    /// Whether the CPU may run above its base clock (turbo / boost).
    pub boost: Option<bool>,
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

    // No fallback to sysinfo: it only knows *current* frequency, and a
    // live reading shown as the maximum is wrong. Unknown beats wrong.
    let max_frequency_mhz = frequency_from_sysfs("cpuinfo_max_freq");

    CpuInfo {
        model,
        architecture,
        physical_cores,
        logical_threads,
        max_frequency_mhz,
        min_frequency_mhz: frequency_from_sysfs("cpuinfo_min_freq"),
        scaling: detect_scaling(),
        virtualization: detect_virtualization(),
        cache: detect_cache(),
    }
}

/// Reads a kHz figure such as `cpuinfo_max_freq` from cpu0's cpufreq
/// directory — the standard sysfs path on virtually every Linux distro.
/// `None` if the kernel doesn't expose cpufreq (some VMs, some ARM
/// boards).
fn frequency_from_sysfs(file: &str) -> Option<u64> {
    let khz: u64 = cpufreq_value(file)?.parse().ok()?;
    Some(khz / 1000)
}

/// Trimmed contents of one cpu0 cpufreq file, `None` if missing or empty.
fn cpufreq_value(file: &str) -> Option<String> {
    read_trimmed(&format!("/sys/devices/system/cpu/cpu0/cpufreq/{file}"))
}

fn read_trimmed(path: &str) -> Option<String> {
    let value = fs::read_to_string(fixture::sys_path(path)).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn detect_scaling() -> FrequencyScaling {
    FrequencyScaling {
        driver: cpufreq_value("scaling_driver"),
        governor: cpufreq_value("scaling_governor"),
        energy_preference: cpufreq_value("energy_performance_preference"),
        boost: detect_boost(),
    }
}

/// `cpufreq/boost` (acpi-cpufreq, amd-pstate) is 1 when boost is on;
/// intel_pstate has no such file and exposes the inverse as `no_turbo`.
fn detect_boost() -> Option<bool> {
    if let Some(boost) = read_trimmed("/sys/devices/system/cpu/cpufreq/boost") {
        return parse_flag(&boost);
    }
    let no_turbo = read_trimmed("/sys/devices/system/cpu/intel_pstate/no_turbo")?;
    parse_flag(&no_turbo).map(|off| !off)
}

fn parse_flag(value: &str) -> Option<bool> {
    match value {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
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
