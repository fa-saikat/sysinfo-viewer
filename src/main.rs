//! JaduPC System Information Viewer
//!
//! A single-window dashboard built with GPUI, styled to match the
//! JaduPC 3-in-1 welcome app: same palette, same full-bleed background +
//! scrim treatment per section, same rem-based responsive scale.
//!
//! Run with: cargo run --release

mod app;
mod colors;
mod tab;

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use gpui::{
    px, size, App, AppContext, AssetSource, Bounds, SharedString, WindowBounds, WindowOptions,
};

use app::SysInfoApp;
use tab::{BackgroundAvailability, Tab};

// ---------------------------------------------------------------------
// Assets — identical strategy to the welcome app: read from an
// `assets/` directory next to the binary (or the crate root in dev)
// rather than embedding, so dropping new background art in doesn't
// require a rebuild.
// ---------------------------------------------------------------------
struct FsAssets {
    root: PathBuf,
}

impl FsAssets {
    fn new() -> Self {
        let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

        let candidates = [
            exe_dir.map(|d| d.join("assets")),
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")),
            Some(PathBuf::from("/usr/share/sysinfo-viewer/assets")),
            Some(PathBuf::from("assets")),
        ];

        let root = candidates
        .into_iter()
        .flatten()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from("assets"));

        Self { root }
    }
}

impl AssetSource for FsAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        let full_path = self.root.join(path);
        match fs::read(&full_path) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(anyhow!(
                "failed to read asset \"{path}\" at {}: {err}",
                full_path.display()
            )),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let dir = self.root.join(path);
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(Vec::new());
        };
        Ok(entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .map(|name| format!("{path}/{name}").into())
        .collect())
    }
}

fn main() {
    let assets = FsAssets::new();
    // Resolved once, up front, against the same root FsAssets reads from
    // — see `tab::BackgroundAvailability` for why.
    let background_availability = BackgroundAvailability::probe(&assets.root);
    // Harness launch switch (see docs/testing.md): opens on the named tab
    // so the screenshot script can capture every tab unattended. Unset or
    // unparsable values open on Overview, exactly as before.
    let initial_tab = std::env::var("SYSINFO_TAB")
        .ok()
        .and_then(|slug| Tab::from_slug(&slug));

    gpui_platform::application()
    .with_assets(assets)
    .run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                       titlebar: None,
                       is_resizable: false,
                       is_movable: true,
                       app_id: Some("SystemInfoViewer".to_string()),
                       window_min_size: Some(size(px(1024.0), px(768.0))),
                       ..Default::default()
            },
            move |_window, cx| {
                cx.new(|_| {
                    let app = SysInfoApp::new(background_availability);
                    match initial_tab {
                        Some(tab) => app.with_tab(tab),
                        None => app,
                    }
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
