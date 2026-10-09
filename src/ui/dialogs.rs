//! About dialog: mark, sentence, facts, close.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt, WindowExt};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, App, Styled, Window};

use super::sidebar::context_tile;
use gpui_kit::assets::IconName;

/// Information dialog for the suite: 40 inverted mark, one sentence, a
/// description list, and a right-aligned Close button.
pub fn show_about(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog, _window, cx| {
        let mark = context_tile(
            IconName::Cpu,
            40.0,
            20.0,
            cx.theme().foreground,
            cx.theme().background,
        );
        let close = Button::new("about-close")
            .primary()
            .small()
            .label("Close")
            .on_click(|_, window, cx| {
                window.close_dialog(cx);
            });
        dialog
            .footer(
                div()
                    .flex()
                    .justify_end()
                    .w_full()
                    .child(close),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(14.))
                            .child(mark)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(div().text_base().font_semibold().child("SysInfo"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("System details for the suite."),
                                    ),
                            ),
                    )
                    .child(
                        DescriptionList::new()
                            .label_width(px(110.))
                            .columns(1)
                            .bordered(true)
                            .item("Version", "1.2.0", 1)
                            .item("Base", "Debian 12 (Bookworm)", 1)
                            .item("Developer", "ShopnoOS team", 1)
                            .item("License", "MIT", 1),
                    ),
            )
    });
}
