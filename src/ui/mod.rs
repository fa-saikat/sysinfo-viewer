//! App shell modules: sidebar, content and dialogs per tab.

pub mod dialogs;
pub mod library;
pub mod root;
pub mod sidebar;
pub mod views;

pub use root::{RootView, KEY_CONTEXT};

gpui_kit::actions!(sysinfo, [FocusSearch]);
