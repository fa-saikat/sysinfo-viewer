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
/// thing with an explicit `0000:` domain prefix.
fn sysfs_pci_path(bus_id: &str) -> String {
    format!("/sys/bus/pci/devices/0000:{bus_id}")
}

fn amdgpu_vram(bus_id: &str) -> Option<VramSize> {
    let bytes: u64 = std::fs::read_to_string(format!("{}/mem_info_vram_total", sysfs_pci_path(bus_id)))
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
