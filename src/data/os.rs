use super::fixture;
use super::format_uptime;
use std::fs;
use sysinfo::System;

pub struct OsInfo {
    pub distro: String,
    pub kernel_version: String,
    pub hostname: String,
    pub uptime: String,
}

pub fn collect() -> OsInfo {
    OsInfo {
        distro: System::long_os_version()
            .or_else(System::name)
            .unwrap_or_else(|| "Unknown distribution".to_string()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown host".to_string()),
        uptime: format_uptime(System::uptime()),
    }
}

/// Machine serial from DMI, when userspace is allowed to see it. Often
/// root-only or filled with a placeholder on VMs, so `None` is routine —
/// the UI says "Unknown" rather than inventing one.
pub fn serial_number() -> Option<String> {
    dmi_value("product_serial")
}

/// Bare-metal product name for the Overview device list. Same honesty
/// rules as the serial: `None` means "couldn't determine".
pub fn product_name() -> Option<String> {
    dmi_value("product_name")
}

fn dmi_value(field: &str) -> Option<String> {
    let raw = fs::read_to_string(fixture::sys_path(&format!("/sys/class/dmi/id/{field}"))).ok()?;
    clean_dmi_value(raw.trim())
}

/// Pure filter over a DMI file body, kept separate so tests can drive it
/// with canned inputs.
fn clean_dmi_value(value: &str) -> Option<String> {
    match value {
        "" | "None" | "Unknown" | "Default string" | "To be filled by O.E.M."
        | "To Be Filled By O.E.M." => None,
        _ => Some(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_placeholder_serials() {
        for placeholder in [
            "",
            "None",
            "Unknown",
            "Default string",
            "To be filled by O.E.M.",
            "To Be Filled By O.E.M.",
        ] {
            assert_eq!(clean_dmi_value(placeholder), None);
        }
        assert_eq!(
            clean_dmi_value("PF2ABC123"),
            Some("PF2ABC123".to_string())
        );
    }
}
