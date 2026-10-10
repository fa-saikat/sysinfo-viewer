//! Full-stack proving loop (ticket #8): backend and frontend tested
//! together, never in isolation.
//!
//! Each test points collection at a synthetic root (`SYSINFO_SYS_ROOT`)
//! plus a stub `bin/` on `PATH`, runs the real `SystemSnapshot::collect`,
//! and asserts on the exact inputs each tab renders: facts, formatted
//! strings, table order, chart points and honest fallbacks. `sysinfo` and
//! `if-addrs` read the live machine, so for those sources the tests assert
//! structural invariants (sorted, well-formed, in-range) rather than fixed
//! values; everything routed through the fixture seams asserts exact values.
//!
//! Environment mutation is process-global: every test holds the serial
//! lock via [`Env`] and restores both variables on drop, even on failure.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use sysinfo_viewer::data::{
    format_bytes, format_frequency_mhz, CpuInfo, GpuInfo, InterfaceState, MemoryInfo,
    NetworkInterface, OsInfo, StorageDevice, SystemSnapshot, Virtualization, VramSize,
};
use sysinfo_viewer::search::{count_matches, matches_query, row_texts, tab_matches};

static SERIAL: Mutex<()> = Mutex::new(());

/// Holds the env lock and restores `PATH` / `SYSINFO_SYS_ROOT` on drop.
struct Env {
    saved_path: Option<OsString>,
    saved_root: Option<OsString>,
    _guard: MutexGuard<'static, ()>,
}

impl Env {
    fn set(root: Option<&Path>, bin: Option<&Path>) -> Self {
        let guard = SERIAL.lock().unwrap();
        let saved_path = std::env::var_os("PATH");
        let saved_root = std::env::var_os("SYSINFO_SYS_ROOT");
        match root {
            Some(dir) => std::env::set_var("SYSINFO_SYS_ROOT", dir),
            None => std::env::remove_var("SYSINFO_SYS_ROOT"),
        }
        match bin {
            Some(dir) => std::env::set_var("PATH", dir),
            None => std::env::remove_var("PATH"),
        }
        Self {
            saved_path,
            saved_root,
            _guard: guard,
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        match &self.saved_path {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
        match &self.saved_root {
            Some(value) => std::env::set_var("SYSINFO_SYS_ROOT", value),
            None => std::env::remove_var("SYSINFO_SYS_ROOT"),
        }
    }
}

/// A synthetic machine root: `proc/`, `sys/` trees plus a stub `bin/`.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/tmp/harness")
            .join(format!("{name}-{}", std::process::id()));
        fs::create_dir_all(root.join("bin")).unwrap();
        Self { root }
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn stub(&self, name: &str, body: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    fn bin(&self) -> PathBuf {
        self.root.join("bin")
    }
}

const CPUINFO_VMX: &str = "processor\t: 0\nmodel name\t: Fixture CPU\nflags\t\t: fpu vme vmx sse2\n\nprocessor\t: 1\nmodel name\t: Fixture CPU\nflags\t\t: fpu vme vmx sse2\n";

const LSPCI_INTEL: &str = "printf '00:02.0 VGA compatible controller: Intel Corporation UHD Graphics 620 (rev 07)\n\\tSubsystem: Fixture Device\\n\\tKernel driver in use: i915\\n'";

fn assert_network_invariants(interfaces: &[NetworkInterface]) {
    let names: Vec<&str> = interfaces.iter().map(|i| i.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "interfaces render sorted by name");
    for iface in interfaces {
        assert!(!iface.name.is_empty());
        assert!(
            iface.ip_address.as_ref().is_none_or(|ip| !ip.is_empty()),
            "empty IP string for {}",
            iface.name
        );
        assert!(matches!(
            iface.state,
            InterfaceState::Up | InterfaceState::Down | InterfaceState::Unknown
        ));
    }
}

fn assert_storage_invariants(devices: &[StorageDevice]) {
    let mounts: Vec<&str> = devices.iter().map(|d| d.mount_point.as_str()).collect();
    let mut sorted = mounts.clone();
    sorted.sort();
    assert_eq!(mounts, sorted, "devices render sorted by mount point");
    for device in devices {
        let fraction = device.used_fraction();
        assert!(
            (0.0..=1.0).contains(&fraction),
            "fraction out of range for {}",
            device.mount_point
        );
        assert_eq!(device.used_bytes(), device.total_bytes.saturating_sub(device.available_bytes));
    }
}

#[test]
fn normal_fixture_collects_every_tab_end_to_end() {
    let fixture = Fixture::new("normal");
    fixture.write("proc/cpuinfo", CPUINFO_VMX);
    fixture.write(
        "sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq",
        "4900000\n",
    );
    fixture.write("sys/devices/system/cpu/cpu0/cache/index0/level", "1\n");
    fixture.write("sys/devices/system/cpu/cpu0/cache/index0/size", "32K\n");
    fixture.write(
        "sys/devices/system/cpu/cpu0/cache/index0/shared_cpu_list",
        "0-1\n",
    );
    fixture.write("sys/devices/system/cpu/cpu0/cache/index1/level", "1\n");
    fixture.write("sys/devices/system/cpu/cpu0/cache/index1/size", "32K\n");
    fixture.write(
        "sys/devices/system/cpu/cpu0/cache/index1/shared_cpu_list",
        "0-1\n",
    );
    fixture.write("sys/devices/system/cpu/cpu0/cache/index2/level", "2\n");
    fixture.write("sys/devices/system/cpu/cpu0/cache/index2/size", "512K\n");
    fixture.write(
        "sys/devices/system/cpu/cpu0/cache/index2/shared_cpu_list",
        "0-1\n",
    );
    fixture.write("sys/devices/system/cpu/cpu0/cache/index3/level", "3\n");
    fixture.write("sys/devices/system/cpu/cpu0/cache/index3/size", "16384K\n");
    fixture.write(
        "sys/devices/system/cpu/cpu0/cache/index3/shared_cpu_list",
        "0-11\n",
    );
    // A second core repeats its private caches and shares the L3: the
    // collector must still report each level once.
    fixture.write("sys/devices/system/cpu/cpu1/cache/index0/level", "1\n");
    fixture.write("sys/devices/system/cpu/cpu1/cache/index0/size", "32K\n");
    fixture.write(
        "sys/devices/system/cpu/cpu1/cache/index0/shared_cpu_list",
        "1\n",
    );
    fixture.write("sys/devices/system/cpu/cpu1/cache/index3/level", "3\n");
    fixture.write("sys/devices/system/cpu/cpu1/cache/index3/size", "16384K\n");
    fixture.write(
        "sys/devices/system/cpu/cpu1/cache/index3/shared_cpu_list",
        "0-11\n",
    );
    fixture.stub("lspci", LSPCI_INTEL);
    let _env = Env::set(Some(&fixture.root), Some(&fixture.bin()));

    let snapshot = SystemSnapshot::collect();

    // Overview inputs: header freshness plus hostname/distro subtitle.
    assert!(!snapshot.age_label().is_empty());
    assert!(!snapshot.os.hostname.is_empty());
    assert!(!snapshot.os.distro.is_empty());
    assert!(!snapshot.os.kernel_version.is_empty());
    assert!(!snapshot.os.uptime.is_empty());

    // Processor inputs: facts plus the chart points the grid consumes.
    assert!(!snapshot.cpu.model.is_empty());
    assert!(!snapshot.cpu.architecture.is_empty());
    assert!(snapshot.cpu.logical_threads >= 1);
    assert_eq!(
        snapshot.cpu.per_core_frequency_mhz.len(),
        snapshot.cpu.logical_threads,
        "one chart point per thread"
    );
    assert_eq!(snapshot.cpu.virtualization, Virtualization::IntelVtx);
    assert_eq!(snapshot.cpu.max_frequency_mhz, Some(4900));
    assert_eq!(format_frequency_mhz(4900), "4.90 GHz");
    let l3 = snapshot
        .cpu
        .cache
        .iter()
        .find(|entry| entry.level == 3)
        .expect("shared L3 from the fixture tree");
    assert_eq!(l3.size_bytes, 16 * 1024 * 1024);
    assert!(snapshot.cpu.cache.windows(2).all(|pair| pair[0].level <= pair[1].level));
    assert_eq!(snapshot.cpu.cache.len(), 3, "L1, L2 and L3 exactly once");

    // Memory inputs: figures plus the bar fractions.
    assert!(snapshot.memory.total_bytes > 0);
    assert!(snapshot.memory.used_bytes <= snapshot.memory.total_bytes);
    assert!((0.0..=1.0).contains(&snapshot.memory.used_fraction()));
    assert_eq!(
        snapshot.memory.has_swap(),
        snapshot.memory.swap_total_bytes > 0
    );
    assert!(!format_bytes(snapshot.memory.total_bytes).is_empty());

    // Network and storage inputs: live sources, structural invariants.
    assert_network_invariants(&snapshot.network);
    assert_storage_invariants(&snapshot.storage);
    assert!(
        !snapshot.storage.is_empty(),
        "expected at least the root mount"
    );

    // Graphics inputs: stubbed tool output parsed into render shape.
    assert_eq!(snapshot.gpus.len(), 1);
    let gpu = &snapshot.gpus[0];
    assert!(gpu.model.contains("UHD Graphics 620"));
    assert_eq!(gpu.driver, "i915");
    assert!(matches!(gpu.vram, VramSize::Shared));
}

#[test]
fn no_tools_fixture_degrades_honestly() {
    let fixture = Fixture::new("no-tools");
    // Deliberately no proc/cpuinfo, no cpufreq tree, no stubs: every
    // routed read misses and every tool lookup fails.
    let _env = Env::set(Some(&fixture.root), Some(&fixture.bin()));

    let snapshot = SystemSnapshot::collect();

    assert!(snapshot.gpus.is_empty(), "no lspci means no GPUs, not fake ones");
    assert!(snapshot.cpu.cache.is_empty());
    for iface in &snapshot.network {
        assert!(matches!(
            iface.iface_type.label(),
            "Loopback" | "Wi-Fi" | "Ethernet" | "Virtual"
        ));
    }
    assert_eq!(
        snapshot.cpu.virtualization.label(),
        "Unknown",
        "unreadable cpuinfo is Unknown, not Unsupported"
    );
    // Live-sourced tabs still collect without panicking.
    assert_network_invariants(&snapshot.network);
    assert_storage_invariants(&snapshot.storage);
    assert!(!snapshot.os.hostname.is_empty());
}

#[test]
fn stubbed_discrete_gpus_resolve_dedicated_vram() {
    let fixture = Fixture::new("discrete");
    fixture.stub(
        "lspci",
        "printf '03:00.0 VGA compatible controller: Advanced Micro Devices, Inc. [AMD/ATI] Navi 33 [Radeon RX 7600] (rev c1)\\n\\tKernel driver in use: amdgpu\\n01:00.0 3D controller: NVIDIA Corporation GA107M [GeForce RTX 3050] (rev a1)\\n\\tKernel driver in use: nvidia\\n'",
    );
    fixture.stub(
        "nvidia-smi",
        "printf '00000000:01:00.0, 4096\\n'",
    );
    fixture.write(
        "sys/bus/pci/devices/0000:03:00.0/mem_info_vram_total",
        "8589934592\n",
    );
    let _env = Env::set(Some(&fixture.root), Some(&fixture.bin()));

    let snapshot = SystemSnapshot::collect();

    assert_eq!(snapshot.gpus.len(), 2);
    let amd = snapshot
        .gpus
        .iter()
        .find(|g| g.driver == "amdgpu")
        .expect("AMD GPU parsed");
    assert!(matches!(amd.vram, VramSize::Dedicated(8589934592)));
    assert_eq!(format_bytes(8589934592), "8.0 GB");
    let nvidia = snapshot
        .gpus
        .iter()
        .find(|g| g.driver == "nvidia")
        .expect("NVIDIA GPU parsed");
    assert!(matches!(nvidia.vram, VramSize::Dedicated(4294967296)));
}

#[test]
fn search_inventory_counts_and_switches() {
    let fixture = Fixture::new("search");
    fixture.write("proc/cpuinfo", CPUINFO_VMX);
    fixture.stub("lspci", LSPCI_INTEL);
    let _env = Env::set(Some(&fixture.root), Some(&fixture.bin()));

    let snapshot = SystemSnapshot::collect();

    // An empty query matches every row of every tab.
    for tab in [
        "overview",
        "processor",
        "memory",
        "network",
        "storage",
        "graphics",
    ] {
        let (visible, total) = count_matches(tab, &snapshot, false, "");
        assert!(total > 0, "row inventory for {tab}");
        assert_eq!(visible, total, "empty query shows all {tab} rows");
    }

    // A nonsense query matches nothing and switches nothing.
    let (visible, _) = count_matches("storage", &snapshot, false, "zzz-no-such-row");
    assert_eq!(visible, 0);
    assert!(!tab_matches("storage", &snapshot, false, "zzz-no-such-row"));

    // Every mount path contains '/', so it matches every storage row.
    if !snapshot.storage.is_empty() {
        let (visible, total) = count_matches("storage", &snapshot, false, "/");
        assert_eq!(visible, total);
    }

    // The live hostname always keeps Overview, case-insensitively.
    assert!(tab_matches(
        "overview",
        &snapshot,
        false,
        &snapshot.os.hostname.to_uppercase()
    ));
    assert!(matches_query("SHOPNO", "shopno"));
}

#[test]
fn graphics_inventory_is_three_rows_per_gpu() {
    // Regression: the graphics renderer filters per-GPU chunks of three
    // inventory rows (Model/VRAM/Driver). The inventory once returned one
    // combined string per GPU, so opening the tab sliced [0..3] out of a
    // length-1 vec and panicked (likewise, typing "gp" auto-switched to
    // the tab and crashed the same way).
    let fixture = Fixture::new("graphics-inventory");
    fixture.stub(
        "lspci",
        "printf '03:00.0 VGA compatible controller: Advanced Micro Devices, Inc. [AMD/ATI] Navi 33 [Radeon RX 7600] (rev c1)\\n\\tKernel driver in use: amdgpu\\n01:00.0 3D controller: NVIDIA Corporation GA107M [GeForce RTX 3050] (rev a1)\\n\\tKernel driver in use: nvidia\\n'",
    );
    fixture.stub(
        "nvidia-smi",
        "printf '00000000:01:00.0, 4096\\n'",
    );
    fixture.write(
        "sys/bus/pci/devices/0000:03:00.0/mem_info_vram_total",
        "8589934592\n",
    );
    let _env = Env::set(Some(&fixture.root), Some(&fixture.bin()));

    let snapshot = SystemSnapshot::collect();
    assert_eq!(snapshot.gpus.len(), 2);

    let texts = row_texts("graphics", &snapshot, false);
    assert_eq!(
        texts.len(),
        snapshot.gpus.len() * 3,
        "one Model/VRAM/Driver row per GPU"
    );
    // Each GPU's chunk carries its own model and driver strings.
    for (index, gpu) in snapshot.gpus.iter().enumerate() {
        let chunk = &texts[index * 3..(index + 1) * 3];
        assert!(
            chunk.iter().any(|text| text.contains(&gpu.model)),
            "chunk {index} carries its model"
        );
        assert!(
            chunk.iter().any(|text| text.contains(&gpu.driver)),
            "chunk {index} carries its driver"
        );
    }
    // The empty query keeps every row, and a driver query keeps its tab.
    let (visible, total) = count_matches("graphics", &snapshot, false, "");
    assert_eq!((visible, total), (6, 6));
    assert!(tab_matches("graphics", &snapshot, false, "amdgpu"));
    assert!(tab_matches("graphics", &snapshot, false, "nvidia"));
}

#[test]
fn render_input_shapes_hold_across_tabs() {
    // Guards the exact shapes each tab's renderer consumes today, so a
    // later data change that would silently unplug a tab fails here first.
    let fixture = Fixture::new("shapes");
    fixture.write("proc/cpuinfo", CPUINFO_VMX);
    fixture.stub("lspci", LSPCI_INTEL);
    let _env = Env::set(Some(&fixture.root), Some(&fixture.bin()));

    let snapshot = SystemSnapshot::collect();

    // Overview subtitle and device rows need all four OS facts.
    let OsInfo {
        distro,
        kernel_version,
        hostname,
        uptime,
    } = &snapshot.os;
    for fact in [distro, kernel_version, hostname, uptime] {
        assert!(!fact.is_empty());
    }
    // Processor grid needs per-core points within a sane bound.
    assert!(!snapshot.cpu.per_core_frequency_mhz.is_empty());
    for mhz in &snapshot.cpu.per_core_frequency_mhz {
        assert!((0..100_000).contains(mhz), "implausible frequency {mhz}");
    }
    // Memory tab needs totals plus the swap branch input.
    let MemoryInfo {
        total_bytes,
        swap_total_bytes,
        ..
    } = snapshot.memory;
    assert!(total_bytes > 0);
    let _swap_branch: bool = swap_total_bytes > 0;
    // Network/storage tables need owned rows the renderers can sort.
    let _: Vec<NetworkInterface> = snapshot.network;
    let _: Vec<StorageDevice> = snapshot.storage;
    // Graphics sheet needs owned GPU entries.
    let _: Vec<GpuInfo> = snapshot.gpus;
    // CPU info is consumed by value in places; it must stay Clone-free
    // but movable — this binding proves the shape.
    let CpuInfo { model, .. } = snapshot.cpu;
    assert!(!model.is_empty());
}
