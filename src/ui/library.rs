//! Shared content primitives: page header, notices, groups, tags.
//!
//! Everything here is pure layout over caller-supplied content. Anything
//! needing a click handler is built by the caller (which owns the view
//! context); the kit has no action slot on `Alert`, so notices with a
//! button use the custom `notice_row` below.

use gpui_kit::assets::IconName;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, StyledExt};
use gpui_kit::prelude::*;
use gpui_kit::{div, white, App, Div, Hsla, Styled, px};

use super::sidebar::context_tile;

/// 52 header tile with the tab glyph.
pub fn header_tile(icon: IconName, cx: &App) -> Div {
    context_tile(icon, 52.0, 26.0, cx.theme().primary, white())
}

/// Section label with an optional muted aside on the right.
pub fn group_title(title: &'static str, aside: Option<String>, cx: &App) -> Div {
    let mut label = div()
        .flex()
        .justify_between()
        .items_baseline()
        .text_xs()
        .font_semibold()
        .text_color(cx.theme().muted_foreground);
    label = label.child(div().child(title));
    if let Some(aside) = aside {
        label = label.child(
            div()
                .font_normal()
                .text_color(cx.theme().muted_foreground)
                .child(aside),
        );
    }
    label
}

/// Neutral fact tag; `warning` for problems that need attention.
pub fn fact_tag(text: String) -> Tag {
    Tag::secondary().small().child(text)
}

/// Determinate progress bar, 0–100. Neutral accent fill; never a status
/// colour unless something can be lost or needs attention.
pub fn progress_bar(id: impl Into<gpui_kit::ElementId>, percent_0_100: f32) -> Progress {
    Progress::new(id).value(percent_0_100.clamp(0.0, 100.0))
}

/// Notice without an action, straight from the kit.
#[allow(dead_code)]
pub fn notice(id: &'static str, lead: &'static str, body: String) -> Alert {
    Alert::warning(id, body).title(lead)
}

/// Notice with an action slot: 28 tone tile, lead line, one sentence,
/// and the caller's button on the right. `tone` is the accent for
/// "you can improve this", `warning` for "something is missing".
pub fn notice_row(
    icon: IconName,
    tone: Hsla,
    lead: &'static str,
    body: String,
    action: Div,
    cx: &App,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .p(px(12.))
        .px(px(16.))
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(tone.opacity(0.35))
        .bg(tone.opacity(0.08))
        .child(context_tile(icon, 28.0, 14.0, tone, white()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_sm().font_semibold().child(lead))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(body),
                ),
        )
        .child(action)
}

/// State badge in the mockup's outline look: tinted fill, tone border,
/// tone text. `Good` (green) for enabled or healthy, `Bad` (red) for
/// disabled. The badge hugs its content by construction.
pub enum BadgeTone {
    Good,
    Bad,
}

pub fn state_badge(text: String, tone: BadgeTone, cx: &App) -> Div {
    let color = match tone {
        BadgeTone::Good => cx.theme().success,
        BadgeTone::Bad => cx.theme().danger,
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .px(px(8.))
        .py(px(2.))
        .rounded(px(6.))
        .border_1()
        .text_xs()
        .text_color(color)
        .border_color(color)
        .bg(color.opacity(0.1))
        .child(text)
}

/// Wraps a cell element so it hugs its content at the row start instead
/// of stretching across a wide value cell.
pub fn hug(element: Div) -> Div {
    div().flex().flex_row().justify_start().child(element)
}

/// Small Lucide glyph in muted, for input prefixes and table adornments.
pub fn small_icon(icon: IconName, cx: &App) -> Icon {
    Icon::new(icon)
        .small()
        .text_color(cx.theme().muted_foreground)
}
