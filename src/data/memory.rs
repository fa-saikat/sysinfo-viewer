use sysinfo::System;

pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

impl MemoryInfo {
    pub fn used_fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f32 / self.total_bytes as f32).clamp(0.0, 1.0)
        }
    }

    pub fn has_swap(&self) -> bool {
        self.swap_total_bytes > 0
    }
}

pub fn collect(sys: &System) -> MemoryInfo {
    // sysinfo 0.31+ reports memory in bytes. If you pin an older sysinfo
    // (pre-0.27ish reported KB), multiply these four fields by 1024.
    MemoryInfo {
        total_bytes: sys.total_memory(),
        used_bytes: sys.used_memory(),
        available_bytes: sys.available_memory(),
        swap_total_bytes: sys.total_swap(),
        swap_used_bytes: sys.used_swap(),
    }
}
