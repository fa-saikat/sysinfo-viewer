//! Brand accent on top of the kit theme.
//!
//! Shelf uses the kit's Default Dark/Light neutrals unchanged and adds one
//! thing: the context accent. This module is that addition — about 50
//! lines that write the accent into the theme so kit components pick it
//! up with no extra code (see DESIGN_SYSTEM 2.2 and 9.3).

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, Window, rgb, white};

/// The single brand accent. Every context in this app shares it: one flat
/// colour for tiles, the selected-row marker, the one primary button,
/// progress and focus. Holds white text on both themes.
pub const BRAND_ACCENT: u32 = 0x5b6ee1;

pub fn accent(hex: u32) -> Hsla {
    Hsla::from(rgb(hex))
}

/// Writes the accent into the theme fields kit components read.
pub fn apply_accent(hex: u32, cx: &mut App) {
    let a = accent(hex);
    let foreground = white();
    Theme::update(cx, |theme| {
        theme.primary = a;
        theme.primary_hover = a.opacity(0.9);
        theme.primary_active = Hsla {
            l: (a.l - 0.1).max(0.0),
            ..a
        };
        theme.primary_foreground = foreground;
        theme.button_primary = a;
        theme.button_primary_hover = a.opacity(0.9);
        theme.button_primary_active = Hsla {
            l: (a.l - 0.1).max(0.0),
            ..a
        };
        theme.button_primary_foreground = foreground;
        theme.ring = a;
        theme.sidebar_primary = a;
        theme.sidebar_primary_foreground = foreground;
        theme.accent = a;
        theme.accent_foreground = foreground;
        theme.progress_bar = a;
    });
}

/// Switches light/dark and re-applies the accent, because a mode change
/// reloads the kit's colours. Wired to the sidebar footer toggle.
pub fn toggle_mode(window: &mut Window, cx: &mut App) {
    let next = if is_dark(cx) {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    };
    Theme::change(next, Some(window), cx);
    apply_accent(BRAND_ACCENT, cx);
}

pub fn is_dark(cx: &App) -> bool {
    matches!(Theme::global(cx).mode, ThemeMode::Dark)
}
