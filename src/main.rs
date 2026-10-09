//! SysInfo on the Shelf stack: kit bootstrap, native window, tab/theme
//! launch switches for the harness.
//!
//! Run with: cargo run --release

mod assets;
mod tab;
mod theme;
mod ui;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{
    px, size, App, AppContext, Bounds, KeyBinding, Window, WindowBounds, WindowOptions,
};

use assets::AppAssets;
use tab::Tab;
use theme::{apply_accent, BRAND_ACCENT};
use ui::{FocusSearch, RootView, KEY_CONTEXT};

fn main() {
    // Harness launch switches (see docs/testing.md): open on the named
    // tab so the screenshot script can capture every tab unattended.
    // Unset or unparsable values open on dark Overview, exactly as before.
    let initial_tab = std::env::var("SYSINFO_TAB")
        .ok()
        .and_then(|slug| Tab::from_slug(&slug));
    let initial_theme = match std::env::var("SYSINFO_THEME")
        .as_deref()
        .unwrap_or("dark")
    {
        "light" => ThemeMode::Light,
        _ => ThemeMode::Dark,
    };

    gpui_kit::application()
        .with_assets(AppAssets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            Theme::change(initial_theme, None, cx);
            apply_accent(BRAND_ACCENT, cx);
            // Native title bar, so sheets hang full-height.
            Theme::update(cx, |theme| {
                theme.sheet.margin_top = px(0.);
            });
            cx.bind_keys([KeyBinding::new("/", FocusSearch, Some(KEY_CONTEXT))]);

            let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(1024.), px(680.))),
                    ..Default::default()
                },
                cx,
                |window: &mut Window, cx: &mut App| {
                    cx.new(|cx| RootView::new(window, cx, initial_tab))
                },
            )
            .expect("failed to open window");

            cx.activate(true);
        });
}
