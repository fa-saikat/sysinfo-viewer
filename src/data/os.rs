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

/// DMI identity for the Overview device list, read once per snapshot
/// (it can't change while the app runs). `None` means "couldn't
/// determine" and the UI says "Unknown" rather than inventing one.
pub struct DeviceInfo {
    /// Bare-metal product name, e.g. "XPS 13 9380".
    pub product_name: Option<String>,
    /// Machine serial, when userspace may see it. Often root-only or a
    /// placeholder on VMs, so `None` is routine.
    pub serial_number: Option<String>,
}

pub fn collect_device() -> DeviceInfo {
    DeviceInfo {
        product_name: dmi_value("product_name"),
        serial_number: dmi_value("product_serial"),
    }
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
