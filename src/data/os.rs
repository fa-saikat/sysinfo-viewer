use super::format_uptime;
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
