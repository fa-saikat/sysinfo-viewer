//! Search inventory: every filterable row's plain text per tab.
//!
//! The toolbar counts and global auto-switch derive from here, and the tab
//! builders match on the same strings they render — one inventory, two
//! consumers. Tabs are identified by slug (see `Tab::slug` on the app
//! side) so this stays free of UI dependencies and testable in `cargo
//! test` without a display.

use super::data::{
    format_bytes, format_frequency_mhz, product_name, serial_number, storage_totals, vram_label,
    InterfaceState, SystemSnapshot,
};

/// Case-insensitive substring match over a row's text. An empty query
/// matches everything, so unfiltered renders stay untouched.
pub fn matches_query(query: &str, haystack: &str) -> bool {
    let needle = query.trim().to_lowercase();
    needle.is_empty() || haystack.to_lowercase().contains(&needle)
}

/// Group titles per tab slug, so a query matching a title keeps its group.
pub fn group_titles(tab: &str) -> &'static [&'static str] {
    match tab {
        "overview" => &["Device", "Software"],
        "processor" => &["Processor", "Per-core frequency"],
        "memory" => &["Memory", "Swap"],
        "network" => &["General", "Interfaces"],
        "storage" => &["Devices"],
        "graphics" => &["Detected GPUs"],
        _ => &[],
    }
}

fn text(label: &str, value: &str) -> String {
    format!("{label} {value}")
}

/// Every filterable row's plain text for a tab, in render order.
pub fn row_texts(tab: &str, snapshot: &SystemSnapshot, serial_shown: bool) -> Vec<String> {
    match tab {
        "overview" => {
            let cpu = &snapshot.cpu;
            let max_freq = cpu
                .max_frequency_mhz
                .map(format_frequency_mhz)
                .unwrap_or_else(|| "Unknown".to_string());
            let (_, storage_total) = storage_totals(&snapshot.storage);
            let storage = if storage_total > 0 {
                format_bytes(storage_total)
            } else {
                "No mounted devices".to_string()
            };
            vec![
                text(
                    "Model",
                    &product_name().unwrap_or_else(|| "Unknown".to_string()),
                ),
                text(
                    "Processor",
                    &format!("{} · {} threads · {}", cpu.model, cpu.logical_threads, max_freq),
                ),
                text("Memory", &format_bytes(snapshot.memory.total_bytes)),
                text(
                    "Graphics",
                    &snapshot
                        .gpus
                        .first()
                        .map(|gpu| gpu.model.clone())
                        .unwrap_or_else(|| "No GPU detected".to_string()),
                ),
                text("Storage", &storage),
                text(
                    "Serial number",
                    &serial_number()
                        .map(|number| {
                            if serial_shown {
                                number
                            } else {
                                "••••••••••".to_string()
                            }
                        })
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                text("Distribution", &snapshot.os.distro),
                text("Kernel", &snapshot.os.kernel_version),
                text("Architecture", &snapshot.cpu.architecture),
                text("Hostname", &snapshot.os.hostname),
                text("Uptime", &snapshot.os.uptime),
            ]
        }
        "processor" => {
            let cpu = &snapshot.cpu;
            let mut rows = vec![
                text("Model", &cpu.model),
                text(
                    "Cores / threads",
                    &format!("{} / {}", cpu.physical_cores, cpu.logical_threads),
                ),
                text(
                    "Max frequency",
                    &cpu.max_frequency_mhz
                        .map(format_frequency_mhz)
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                text("Architecture", &cpu.architecture),
                text("Virtualization", cpu.virtualization.label()),
            ];
            if let Some(l3) = cpu.cache.iter().find(|entry| entry.level == 3) {
                rows.push(text("Cache", &l3.label()));
            }
            rows.extend(
                cpu.per_core_frequency_mhz
                    .iter()
                    .enumerate()
                    .map(|(index, mhz)| format!("Core {index} {mhz} MHz")),
            );
            rows
        }
        "memory" => {
            let mem = &snapshot.memory;
            vec![
                text("Total", &format_bytes(mem.total_bytes)),
                text("Used", &format_bytes(mem.used_bytes)),
                text("Available", &format_bytes(mem.available_bytes)),
                text("Swap total", &format_bytes(mem.swap_total_bytes)),
                text("Swap used", &format_bytes(mem.swap_used_bytes)),
            ]
        }
        "network" => {
            let mut rows = vec![
                text("Hostname", &snapshot.os.hostname),
                text(
                    "Status",
                    if is_connected(snapshot) {
                        "Connected"
                    } else {
                        "Offline"
                    },
                ),
            ];
            rows.extend(snapshot.network.iter().map(|iface| {
                format!(
                    "{} {} {} {} {}",
                    iface.name,
                    iface.iface_type.label(),
                    iface.ip_address.as_deref().unwrap_or("—"),
                    if iface.mac_address.is_empty() {
                        "—"
                    } else {
                        &iface.mac_address
                    },
                    iface.state.label()
                )
            }));
            rows
        }
        "storage" => snapshot
            .storage
            .iter()
            .map(|device| {
                format!(
                    "{} {} {} {} / {}",
                    device.mount_point,
                    device.device_name,
                    device.filesystem,
                    format_bytes(device.used_bytes()),
                    format_bytes(device.total_bytes)
                )
            })
            .collect(),
        "graphics" => {
            if snapshot.gpus.is_empty() {
                vec!["No GPU details yet".to_string()]
            } else {
                let total = snapshot.memory.total_bytes;
                snapshot
                    .gpus
                    .iter()
                    .flat_map(|gpu| {
                        vec![
                            text("Model", &gpu.model),
                            text("VRAM", &vram_label(&gpu.vram, total)),
                            text("Driver", &gpu.driver),
                        ]
                    })
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

/// Whether any non-loopback interface is up with an address. Shared by
/// the Network status row and the search inventory.
pub fn is_connected(snapshot: &SystemSnapshot) -> bool {    snapshot.network.iter().any(|iface| {
        iface.state == InterfaceState::Up && iface.name != "lo" && iface.ip_address.is_some()
    })
}

/// Visible/total filterable rows for the toolbar count.
pub fn count_matches(
    tab: &str,
    snapshot: &SystemSnapshot,
    serial_shown: bool,
    query: &str,
) -> (usize, usize) {
    let texts = row_texts(tab, snapshot, serial_shown);
    let visible = texts
        .iter()
        .filter(|text| matches_query(query, text))
        .count();
    (visible, texts.len())
}

/// Whether a tab has anything to show for a query: a matching row or a
/// matching group title. Drives global-search auto-switch.
pub fn tab_matches(tab: &str, snapshot: &SystemSnapshot, serial_shown: bool, query: &str) -> bool {
    let (visible, _) = count_matches(tab, snapshot, serial_shown, query);
    visible > 0
        || group_titles(tab)
            .iter()
            .any(|title| matches_query(query, title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(matches_query("", "anything at all"));
        assert!(matches_query("   ", "anything at all"));
    }

    #[test]
    fn matching_is_case_insensitive_substring() {
        assert!(matches_query("nvme", "Samsung NVMe SSD"));
        assert!(matches_query("SHOPNO", "shopno"));
        assert!(!matches_query("zzz-no-match", "shopno · distro"));
    }

    #[test]
    fn unknown_slugs_match_nothing() {
        assert!(group_titles("nope").is_empty());
    }
}
