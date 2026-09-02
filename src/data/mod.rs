//! System data collection, kept entirely separate from rendering.
//!
//! `SystemSnapshot::collect()` is the one entry point the UI layer calls.
//! Every field is best-effort: on a platform quirk, a missing sysfs file,
//! or a command that isn't installed, individual fields degrade to
//! `"Unknown"` / `None` rather than the whole snapshot failing. Linux is
//! the only target (per the brief), so paths under `/proc` and `/sys` are
//! used directly instead of reaching for a cross-platform crate for
//! everything.

mod cpu;
mod gpu;
mod memory;
mod network;
mod os;
mod storage;

pub use cpu::CpuInfo;
pub use gpu::{GpuInfo, VramSize};
pub use memory::MemoryInfo;
pub use network::{InterfaceState, NetworkInterface};
pub use os::OsInfo;
pub use storage::StorageDevice;

use std::time::Instant;
use sysinfo::System;

pub struct SystemSnapshot {
    pub os: OsInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub network: Vec<NetworkInterface>,
    pub storage: Vec<StorageDevice>,
    pub gpus: Vec<GpuInfo>,
    pub collected_at: Instant,
}

impl SystemSnapshot {
    pub fn collect() -> Self {
        // A single `System` refresh backs the OS/CPU/memory collectors so
        // we don't pay sysinfo's refresh cost three times over. Storage,
        // network, and GPU info come from their own dedicated sources
        // (Disks/Networks, /sys, lspci) since sysinfo's `System` doesn't
        // carry them.
        let mut sys = System::new_all();
        sys.refresh_all();

        Self {
            os: os::collect(),
            cpu: cpu::collect(&sys),
            memory: memory::collect(&sys),
            network: network::collect(),
            storage: storage::collect(),
            gpus: gpu::collect(),
            collected_at: Instant::now(),
        }
    }

    /// "Just now" / "12s ago" / "4m ago" for the header's freshness label.
    pub fn age_label(&self) -> String {
        let secs = self.collected_at.elapsed().as_secs();
        match secs {
            0..=1 => "Just now".to_string(),
            2..=59 => format!("{secs}s ago"),
            60..=3599 => format!("{}m ago", secs / 60),
            _ => format!("{}h ago", secs / 3600),
        }
    }
}

// -- shared formatting helpers, used by more than one collector/view -----

/// Bytes -> a human string with one decimal place, picking the largest
/// unit that keeps the number readable (matches how the mockups showed
/// "10.2 GB", "476 GB", etc).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// MHz -> "4.37 GHz" / "800 MHz", matching the frequency formatting used
/// throughout the Processor tab.
pub fn format_frequency_mhz(mhz: u64) -> String {
    if mhz >= 1000 {
        format!("{:.2} GHz", mhz as f64 / 1000.0)
    } else {
        format!("{mhz} MHz")
    }
}

/// Seconds -> "2d 6h 14m", matching the Overview tab's uptime field.
pub fn format_uptime(total_secs: u64) -> String {
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let minutes = (total_secs % 3_600) / 60;

    match (days, hours) {
        (0, 0) => format!("{minutes}m"),
        (0, _) => format!("{hours}h {minutes}m"),
        (_, _) => format!("{days}d {hours}h {minutes}m"),
    }
}
