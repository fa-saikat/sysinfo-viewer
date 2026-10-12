//! Sidebar tabs: the app's six contexts, jumped to directly.
//!
//! Background art is gone on purpose — Shelf stays flat so the content
//! carries the colour, and this app's content is text, not artwork.

use gpui_kit::assets::IconName;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Processor,
    Memory,
    Network,
    Storage,
    Graphics,
}

pub const TABS: [Tab; 6] = [
    Tab::Overview,
    Tab::Processor,
    Tab::Memory,
    Tab::Network,
    Tab::Storage,
    Tab::Graphics,
];

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Processor => "Processor",
            Tab::Memory => "Memory",
            Tab::Network => "Network",
            Tab::Storage => "Storage",
            Tab::Graphics => "Graphics",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Tab::Overview => "System summary",
            Tab::Processor => "Detailed CPU information",
            Tab::Memory => "RAM and swap usage",
            Tab::Network => "Interfaces and connectivity",
            Tab::Storage => "Mounted devices and usage",
            Tab::Graphics => "Detected GPUs",
        }
    }

    /// One Lucide glyph per tab, always rendered white on the accent tile.
    pub fn icon(self) -> IconName {
        match self {
            Tab::Overview => IconName::Info,
            Tab::Processor => IconName::Cpu,
            Tab::Memory => IconName::MemoryStick,
            Tab::Network => IconName::Globe,
            Tab::Storage => IconName::Database,
            Tab::Graphics => IconName::Monitor,
        }
    }

    /// Stable slug identifying the tab outside the UI layer: search
    /// inventory, launch switches and tests all key off this string.
    pub fn slug(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Processor => "processor",
            Tab::Memory => "memory",
            Tab::Network => "network",
            Tab::Storage => "storage",
            Tab::Graphics => "graphics",
        }
    }

    /// Parses the `SYSINFO_TAB` launch switch used by the test harness's
    /// screenshot script. Accepts the tab label plus a few short aliases;
    /// anything else is `None` and the app opens on Overview as usual.
    pub fn from_slug(slug: &str) -> Option<Tab> {
        match slug.trim().to_lowercase().as_str() {
            "overview" => Some(Tab::Overview),
            "processor" | "cpu" => Some(Tab::Processor),
            "memory" | "mem" | "ram" => Some(Tab::Memory),
            "network" | "net" => Some(Tab::Network),
            "storage" | "disk" | "disks" => Some(Tab::Storage),
            "graphics" | "gpu" => Some(Tab::Graphics),
            _ => None,
        }
    }
}
