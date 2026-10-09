//! GPU detection. There's no equivalent of `sysinfo` for graphics
//! hardware on Linux, so this shells out to `lspci` for model + PCI bus
//! ID, resolves the kernel driver from the same output, then goes
//! vendor-specific for VRAM:
//!
//! - AMD (`amdgpu`): reads `mem_info_vram_total` from the matching sysfs
//!   PCI device directory. Exact and free.
//! - NVIDIA (`nvidia`): shells out to `nvidia-smi`, matched back to the
//!   same PCI bus ID so multi-GPU systems don't get scrambled.
//! - Intel (`i915`) and anything else: reported as shared system memory,
//!   since integrated GPUs don't have dedicated VRAM to query.
//!
//! Every step degrades gracefully — no `lspci` binary, no permissions on
//! a sysfs file, no `nvidia-smi` installed, all fall back to `Unknown`
//! rather than panicking or silently dropping the GPU entirely. This is
//! the part of the whole app most worth testing against your actual
//! hardware; PCI/sysfs output varies more across machines than anything
//! else here.

use super::fixture;
use std::process::Command;

#[derive(Clone)]
pub enum VramSize {
    Dedicated(u64),
    /// Integrated GPUs share system RAM rather than owning a fixed pool.
    Shared,
    Unknown,
}

pub struct GpuInfo {
    pub model: String,
    pub driver: String,
    pub vram: VramSize,
    pub pci_bus_id: String,
}

/// Display form for a VRAM size. Integrated GPUs share system RAM, so the
/// honest amount beside "Shared" is the pool it is shared with: total
/// system memory, measured by the caller — never a placeholder.
pub fn vram_label(vram: &VramSize, system_total_bytes: u64) -> String {
    use super::format_bytes;
    match vram {
        VramSize::Dedicated(bytes) => format_bytes(*bytes),
        VramSize::Shared => format!("{} (Shared)", format_bytes(system_total_bytes)),
        VramSize::Unknown => "Unknown".to_string(),
    }
}

pub fn collect() -> Vec<GpuInfo> {
    let Some(lspci_output) = run(&["lspci", "-k"]) else {
        return Vec::new();
    };

    parse_lspci(&lspci_output)
        .into_iter()
        .map(|mut gpu| {
            gpu.vram = resolve_vram(&gpu);
            gpu
        })
        .collect()
}

/// Parses `lspci -k` blocks like:
/// ```text
/// 00:02.0 VGA compatible controller: Intel Corporation UHD Graphics 620 (rev 07)
///         Subsystem: ...
///         Kernel driver in use: i915
/// ```
fn parse_lspci(output: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let is_display_device = ["VGA compatible controller:", "3D controller:", "Display controller:"]
            .iter()
            .any(|marker| line.contains(marker));

        if !line.starts_with(char::is_whitespace) && is_display_device {
            let pci_bus_id = line.split_whitespace().next().unwrap_or("").to_string();
            let model = line
                .split_once(':')
                .and_then(|(_, rest)| rest.split_once(':'))
                .map(|(_, model)| strip_revision_suffix(model.trim()))
                .unwrap_or_else(|| "Unknown GPU".to_string());

            let mut driver = "Unknown".to_string();
            let mut j = i + 1;
            while j < lines.len() && lines[j].starts_with(char::is_whitespace) {
                if let Some(d) = lines[j].trim().strip_prefix("Kernel driver in use:") {
                    driver = d.trim().to_string();
                }
                j += 1;
            }

            gpus.push(GpuInfo {
                model,
                driver,
                vram: VramSize::Unknown,
                pci_bus_id,
            });
            i = j;
        } else {
            i += 1;
        }
    }

    gpus
}

fn strip_revision_suffix(model: &str) -> String {
    match model.rfind(" (rev ") {
        Some(idx) => model[..idx].to_string(),
        None => model.to_string(),
    }
}

fn resolve_vram(gpu: &GpuInfo) -> VramSize {
    match gpu.driver.as_str() {
        "amdgpu" | "radeon" => amdgpu_vram(&gpu.pci_bus_id).unwrap_or(VramSize::Unknown),
        "nvidia" => nvidia_vram(&gpu.pci_bus_id).unwrap_or(VramSize::Unknown),
        "i915" | "xe" => VramSize::Shared, // Intel integrated
        _ => VramSize::Unknown,
    }
}

/// `lspci` bus IDs look like `00:02.0`; sysfs PCI device dirs are the same
/// thing with an explicit `0000:` domain prefix. Re-rooted under the
/// fixture root in tests via [`fixture::sys_path`].
fn sysfs_pci_path(bus_id: &str) -> std::path::PathBuf {
    fixture::sys_path(&format!("/sys/bus/pci/devices/0000:{bus_id}"))
}

fn amdgpu_vram(bus_id: &str) -> Option<VramSize> {
    let bytes: u64 = std::fs::read_to_string(sysfs_pci_path(bus_id).join("mem_info_vram_total"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(VramSize::Dedicated(bytes))
}

fn nvidia_vram(bus_id: &str) -> Option<VramSize> {
    let output = run(&[
        "nvidia-smi",
        "--query-gpu=pci.bus_id,memory.total",
        "--format=csv,noheader,nounits",
    ])?;

    // nvidia-smi reports bus IDs as e.g. "00000000:00:02.0"; compare on
    // the trailing "bus:slot.func" so it matches lspci's shorter form.
    for line in output.lines() {
        let mut parts = line.split(',').map(str::trim);
        let (Some(nv_bus), Some(mib)) = (parts.next(), parts.next()) else {
            continue;
        };
        if nv_bus.ends_with(bus_id) {
            let mib: u64 = mib.parse().ok()?;
            return Some(VramSize::Dedicated(mib * 1024 * 1024));
        }
    }
    None
}

fn run(argv: &[&str]) -> Option<String> {
    let [cmd, args @ ..] = argv else {
        return None;
    };
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LSPCI_FIXTURE: &str = "\
00:02.0 VGA compatible controller: Intel Corporation UHD Graphics 620 (rev 07)\n\
\tSubsystem: Lenovo Device 22b0\n\
\tKernel driver in use: i915\n\
01:00.0 3D controller: NVIDIA Corporation GP108M [GeForce MX150] (rev a1)\n\
\tSubsystem: Lenovo Device 22b1\n\
\tKernel driver in use: nvidia\n\
02:00.0 Network controller: Intel Corporation Wi-Fi 6 AX200\n\
\tKernel driver in use: iwlwifi\n\
03:00.0 Display controller: Advanced Micro Devices, Inc. [AMD/ATI] Vega without driver\n";

    #[test]
    fn parses_display_devices_and_skips_the_rest() {
        let gpus = parse_lspci(LSPCI_FIXTURE);
        assert_eq!(gpus.len(), 3);
        assert_eq!(gpus[0].pci_bus_id, "00:02.0");
        assert_eq!(gpus[0].model, "Intel Corporation UHD Graphics 620");
        assert_eq!(gpus[0].driver, "i915");
        assert_eq!(gpus[1].model, "NVIDIA Corporation GP108M [GeForce MX150]");
        assert_eq!(gpus[1].driver, "nvidia");
    }

    #[test]
    fn missing_driver_falls_back_to_unknown() {
        let gpus = parse_lspci(LSPCI_FIXTURE);
        assert_eq!(gpus[2].driver, "Unknown");
    }

    #[test]
    fn vram_label_names_the_shared_pool() {
        assert_eq!(
            vram_label(&VramSize::Dedicated(8589934592), 0),
            "8.0 GB".to_string()
        );
        assert_eq!(
            vram_label(&VramSize::Shared, 30_7 * 1024 * 1024 * 1024 / 10),
            "30.7 GB (Shared)".to_string()
        );
        assert_eq!(vram_label(&VramSize::Unknown, 0), "Unknown".to_string());
    }

    #[test]
    fn strips_revision_suffix_only() {        assert_eq!(
            strip_revision_suffix("UHD Graphics 620 (rev 07)"),
            "UHD Graphics 620"
        );
        assert_eq!(
            strip_revision_suffix("GeForce MX150 (rev a1)"),
            "GeForce MX150"
        );
        // Parenthesised model names that are not revisions survive.
        assert_eq!(
            strip_revision_suffix("Radeon (TM) Graphics"),
            "Radeon (TM) Graphics"
        );
    }
}
