//! Sidebar tabs. Plays the same role `Slide` played in the welcome app,
//! minus the linear next/prev navigation — this app is a dashboard, not
//! an onboarding flow, so tabs are jumped to directly from the sidebar.

use std::path::Path;

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
    pub fn index(self) -> usize {
        TABS.iter().position(|t| *t == self).unwrap_or(0)
    }

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

    pub fn subtitle(self) -> &'static str {
        match self {
            Tab::Overview => "jadupc-desktop",
            Tab::Processor => "Detailed CPU information",
            Tab::Memory => "RAM and swap usage",
            Tab::Network => "Interfaces and connectivity",
            Tab::Storage => "Mounted devices and usage",
            Tab::Graphics => "Detected GPUs",
        }
    }

    /// Tabler-style glyph placeholder. Swap for `img("icons/....svg")`
    /// once icon assets exist, the same way `background_path()` below
    /// expects real art in `assets/backgrounds/`.
    pub fn icon_glyph(self) -> &'static str {
        match self {
            Tab::Overview => "󰨇",
            Tab::Processor => "",
            Tab::Memory => "󰍛",
            Tab::Network => "",
            Tab::Storage => "",
            Tab::Graphics => "󱄄",
        }
    }

    /// Background image, relative to the `assets/` root — same convention
    /// as `Slide::background_path()` in the welcome app.
    pub fn background_path(self) -> &'static str {
        match self {
            Tab::Overview => "backgrounds/overview.png",
            Tab::Processor => "backgrounds/processor.png",
            Tab::Memory => "backgrounds/memory.png",
            Tab::Network => "backgrounds/network.png",
            Tab::Storage => "backgrounds/storage.png",
            Tab::Graphics => "backgrounds/graphics.png",
        }
    }
}

/// Which tabs currently have a background image on disk, resolved once at
/// startup against the same assets root `FsAssets` reads from.
///
/// The welcome app never needed this: all six of its slide backgrounds
/// shipped together. This app's tabs are more likely to gain artwork
/// piecemeal, so a missing file falls back to a flat
/// `colors::BACKGROUND_FALLBACK` fill (see `SysInfoApp::background()`)
/// instead of asking GPUI to render a broken image.
#[derive(Clone, Copy)]
pub struct BackgroundAvailability {
    present: [bool; TABS.len()],
}

impl BackgroundAvailability {
    /// `assets_root` should be the same directory `FsAssets` resolves to
    /// (see `main.rs`), so this check and the actual asset load agree.
    pub fn probe(assets_root: &Path) -> Self {
        let mut present = [false; TABS.len()];
        for (i, tab) in TABS.iter().enumerate() {
            present[i] = assets_root.join(tab.background_path()).is_file();
        }
        Self { present }
    }

    pub fn has_image(&self, tab: Tab) -> bool {
        self.present[tab.index()]
    }
}
