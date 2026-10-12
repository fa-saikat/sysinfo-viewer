//! Icon registry.
//!
//! Every UI icon comes from Lucide, so the app matches the kit's own
//! components. The kit's default bundle only covers about 100 icons, so
//! anything else is listed once here: an unregistered icon renders as
//! nothing, with no error, so check each one on screen when adding it.
//! `AppAssets` checks these first, then falls back to the kit's full
//! catalog, and is the source handed to `application()`.

use anyhow::Result;
use gpui_kit::assets::{AllAssets, icon_assets};
use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

icon_assets!(
    ExtraIcons,
    [
        Info,
        Cpu,
        MemoryStick,
        Globe,
        Database,
        Monitor,
        Search,
        Copy,
        RefreshCw,
        Moon,
        Sun,
        TriangleAlert
    ]
);

/// Asset source for the whole app: extra icons first, then the kit's
/// full Lucide catalog.
#[derive(Clone, Copy, Default)]
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        AllAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out = ExtraIcons.list(path)?;
        out.extend(AllAssets.list(path)?);
        Ok(out)
    }
}
