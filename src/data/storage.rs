use sysinfo::Disks;

pub struct StorageDevice {
    pub device_name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

impl StorageDevice {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn used_fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes() as f32 / self.total_bytes as f32).clamp(0.0, 1.0)
        }
    }
}

pub fn collect() -> Vec<StorageDevice> {
    let disks = Disks::new_with_refreshed_list();

    let mut devices: Vec<StorageDevice> = disks
        .iter()
        .map(|disk| StorageDevice {
            device_name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            filesystem: disk.file_system().to_string_lossy().to_string(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        })
        .collect();

    devices.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    devices
}
