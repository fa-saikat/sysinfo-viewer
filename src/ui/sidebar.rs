//! Sidebar: brand mark, context rows, footer utilities.
//!
//! The kit's `SidebarMenuItem` only accepts a plain icon, not a tile, so
//! rows are a custom `NavItem` implementing `SidebarItem`. The tile is the
//! signature element: a solid accent square with a white glyph.

use gpui_kit::assets::IconName;
use gpui_kit::component::sidebar::SidebarItem;
use gpui_kit::component::{ActiveTheme, Collapsible, Icon, Sizable, StyledExt};
use gpui_kit::prelude::*;
use gpui_kit::{
    div, px, white, App, Div, ElementId, Entity, Hsla, Styled, Window,
};

use super::root::RootView;
use crate::tab::Tab;

pub const SIDEBAR_WIDTH: f32 = 240.0;

/// A solid accent square with a white monochrome glyph. Never decoration:
/// it stands for a context (`fill` = accent) or the brand (`fill` =
/// foreground, glyph = background).
pub fn context_tile(icon: IconName, tile_px: f32, glyph_px: f32, fill: Hsla, glyph: Hsla) -> Div {
    div()
        .size(px(tile_px))
        .rounded(px((tile_px * 0.24).round()))
        .bg(fill)
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(icon)
                .with_size(px(glyph_px))
                .text_color(glyph),
        )
}

/// Brand mark: 32 inverted tile plus bold name and muted version.
pub fn brand(tile: Hsla, glyph: Hsla, muted: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(6.))
        .pb(px(14.))
        .child(context_tile(IconName::Cpu, 32.0, 18.0, tile, glyph))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui_kit::FontWeight::BOLD)
                        .child("SysInfo"),
                )
                .child(div().text_xs().text_color(muted).child("v1.2.0")),
        )
}

/// Sidebar rows: context tabs plus the group label between them.
#[derive(Clone)]
pub enum NavItem {
    Tab {
        tab: Tab,
        active: bool,
        view: Entity<RootView>,
    },
    Group(&'static str),
}

impl Collapsible for NavItem {
    fn collapsed(self, _collapsed: bool) -> Self {
        // The sidebar never collapses, so there is nothing to remember.
        self
    }

    fn is_collapsed(&self) -> bool {
        false
    }
}

impl SidebarItem for NavItem {
    fn render(
        self,
        id: impl Into<ElementId>,
        _window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        match self {
            NavItem::Group(label) => div()
                .id(id)
                .px(px(8.))
                .pt(px(10.))
                .pb(px(4.))
                .text_size(px(11.))
                .font_semibold()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_uppercase())
                .into_any_element(),
            NavItem::Tab { tab, active, view } => {
                let accent = cx.theme().primary;
                let mut row = div()
                    .id(id)
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(8.))
                    .my(px(3.))
                    .h(px(36.))
                    .rounded(cx.theme().radius)
                    .cursor_pointer()
                    .child(context_tile(tab.icon(), 24.0, 14.0, accent, white()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .truncate()
                            .text_color(cx.theme().sidebar_foreground)
                            .child(tab.label()),
                    )
                    .on_click(move |_, _, cx: &mut App| {
                        view.update(cx, |this, cx| this.set_tab(tab, cx));
                    });
                if active {
                    row = row
                        .bg(cx.theme().sidebar_accent)
                        .child(
                            div()
                                .absolute()
                                .left(px(-10.))
                                .top(px(10.))
                                .bottom(px(10.))
                                .w(px(3.))
                                .rounded(px(2.))
                                .bg(accent),
                        )
                        .font_medium();
                } else {
                    row = row.hover(|style| style.bg(cx.theme().sidebar_accent));
                }
                row.into_any_element()
            }
        }
    }
}
