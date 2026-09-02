//! Shared color palette. Deliberately identical to the welcome app's
//! `mod colors` so the two apps read as one product family.
//!
//! Not every token here is used by this app yet (e.g. `PANEL_BORDER`,
//! `ACCENT_SOFT`) — kept anyway so the palette stays a drop-in match with
//! the welcome app's rather than a partial copy.
#![allow(dead_code)]

pub const PANEL: u32 = 0x14141c;
pub const PANEL_BORDER: u32 = 0x2a2a35;
pub const TEXT_PRIMARY: u32 = 0xf5f5f7;
pub const TEXT_SECONDARY: u32 = 0x9a9aa5;
pub const TEXT_MUTED: u32 = 0x6b6b76;
pub const ACCENT: u32 = 0x8b5cf6;
pub const ACCENT_SOFT: u32 = 0x6d3fd6;
pub const SUCCESS: u32 = 0x38d97a;
pub const SUCCESS_BG: u32 = 0x0f2018;

// -- new tokens, needed because (unlike the six full-bleed onboarding
// slides) this app's cards sit on top of background art *and* have to
// remain legible when that art is missing --------------------------------

/// Full-bleed background scrim, same job as the welcome app's
/// `rgba(0x00000020)` overlay, just a little heavier since dashboard text
/// density is higher than a single onboarding headline.
pub const SCRIM: u32 = 0x00000038;

/// Content card fill over a background image — the same "translucent
/// panel punched through the art" trick as the welcome app's hotkey chips
/// (`rgba(0x0000004d)`), tuned so art still reads through at the edges.
pub const CARD_BG: u32 = 0x0a0a0f8c;

/// Sidebar fill — a touch more opaque than content cards. It's mostly
/// text and icons, not artwork, so it needs steadier contrast regardless
/// of what's behind it.
pub const SIDEBAR_BG: u32 = 0x0a0a0fb3;

/// Border color for cards drawn over background art. Same hue as
/// `PANEL_BORDER`, just carried as rgba so it doesn't go fully opaque
/// against light artwork.
pub const CARD_BORDER: u32 = 0x2a2a35b3;

/// Fallback fill used in place of a missing background image (see
/// `crate::tab::BackgroundAvailability`) — flat, no image, no scrim.
pub const BACKGROUND_FALLBACK: u32 = PANEL;
