use std::collections::HashMap;
use sysinfo::Disks;

#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Key identifying one underlying volume. Btrfs subvolumes (and bind
/// mounts) surface as several mount points backed by the same device
/// with identical totals — summing them naively double-counts a 512 GB
/// SSD into ~860 GB. An empty device name falls back to the mount
/// point so pseudo mounts never collapse into one entry.
pub fn dedup_key(device: &StorageDevice) -> String {
    if device.device_name.is_empty() {
        device.mount_point.clone()
    } else {
        device.device_name.clone()
    }
}

/// One entry per underlying volume, sorted by mount point for stable
/// rendering. When readings for the same device disagree, the entry
/// with the largest total wins so totals never undercount.
pub fn unique_devices(devices: &[StorageDevice]) -> Vec<&StorageDevice> {
    let mut by_device: HashMap<String, &StorageDevice> = HashMap::new();
    for device in devices {
        let key = dedup_key(device);
        match by_device.get(&key) {
            Some(existing) if existing.total_bytes >= device.total_bytes => {}
            _ => {
                by_device.insert(key, device);
            }
        }
    }
    let mut unique: Vec<&StorageDevice> = by_device.into_values().collect();
    unique.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    unique
}

/// `(used, total)` summed over unique devices, not mount points.
/// Use this everywhere a headline figure is shown (header tag,
/// Overview) so btrfs subvolumes don't inflate the numbers.
pub fn storage_totals(devices: &[StorageDevice]) -> (u64, u64) {
    let unique = unique_devices(devices);
    let total = unique.iter().map(|d| d.total_bytes).sum();
    let used = unique.iter().map(|d| d.used_bytes()).sum();
    (used, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, mount: &str, fs: &str, total: u64, available: u64) -> StorageDevice {
        StorageDevice {
            device_name: name.to_string(),
            mount_point: mount.to_string(),
            filesystem: fs.to_string(),
            total_bytes: total,
            available_bytes: available,
        }
    }

    #[test]
    fn totals_dedupe_btrfs_subvolumes_on_one_device() {
        let gb: u64 = 1024 * 1024 * 1024;
        let devices = vec![
            device("/dev/nvme0n1p3", "/", "btrfs", 100 * gb, 44 * gb),
            device("/dev/nvme0n1p3", "/root", "btrfs", 100 * gb, 44 * gb),
            device("/dev/nvme0n1p3", "/srv", "btrfs", 100 * gb, 44 * gb),
            device("/dev/nvme0n1p4", "/home", "btrfs", 360 * gb, 197 * gb),
            device("/dev/nvme0n1p1", "/boot/efi", "vfat", 1 * gb, 1 * gb),
        ];
        let (used, total) = storage_totals(&devices);
        assert_eq!(total, (100 + 360 + 1) * gb);
        assert_eq!(used, (56 + 163) * gb);
    }

    #[test]
    fn empty_device_names_never_collapse() {
        let devices = vec![
            device("", "/run", "tmpfs", 8, 4),
            device("", "/dev/shm", "tmpfs", 8, 4),
        ];
        let (used, total) = storage_totals(&devices);
        assert_eq!(total, 16);
        assert_eq!(used, 8);
        assert_eq!(unique_devices(&devices).len(), 2);
    }
}
