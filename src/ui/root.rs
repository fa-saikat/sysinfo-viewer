//! App shell: state, actions, page header, toolbar and render.
//!
//! The content column is header → notice → toolbar → tab body. Handlers
//! capture the view entity and update through it, which keeps every
//! callback working from `App`-only contexts such as sidebar rows.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::sidebar::Sidebar;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt, WindowExt};
use gpui_kit::prelude::*;
use gpui_kit::{
    div, px, ClipboardItem, Context, Div, Entity, FocusHandle, Render, Styled, Window,
};

use super::dialogs;
use super::library::header_tile;
use super::sidebar::{brand, NavItem, SIDEBAR_WIDTH};
use super::views;
use crate::tab::{Tab, TABS};
use crate::theme;
use sysinfo_viewer::data::{serial_number, SystemSnapshot};

pub const KEY_CONTEXT: &str = "SysInfo";

pub struct RootView {
    focus: FocusHandle,
    tab: Tab,
    snapshot: SystemSnapshot,
    serial_shown: bool,
    lspci_missing: bool,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, initial: Option<Tab>) -> Self {
        let _ = window;
        Self {
            focus: cx.focus_handle(),
            tab: initial.unwrap_or(Tab::Overview),
            snapshot: SystemSnapshot::collect(),
            serial_shown: false,
            lspci_missing: lspci_missing(),
        }
    }

    pub fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.snapshot = SystemSnapshot::collect();
        self.lspci_missing = lspci_missing();
        cx.notify();
    }

    fn toggle_serial(&mut self, cx: &mut Context<Self>) {
        self.serial_shown = !self.serial_shown;
        cx.notify();
    }

    fn subtitle(&self) -> String {
        match self.tab {
            Tab::Overview => format!("{} · {}", self.snapshot.os.hostname, self.snapshot.os.distro),
            _ => self.tab.subtitle().to_string(),
        }
    }

    fn toolbar_title(&self) -> String {
        match self.tab {
            Tab::Overview => "Details".to_string(),
            _ => format!("{} details", self.tab.label()),
        }
    }

    fn header(&self, view: &Entity<Self>, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let stats = views::header_stats(self.tab, &self.snapshot, cx);
        let mut actions = div().flex().items_center().gap(px(8.));
        actions = actions.child(
            Button::new("header-refresh")
                .outline()
                .icon(IconName::RefreshCw)
                .tooltip("Refresh snapshot")
                .on_click({
                    let view = view.clone();
                    move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.refresh(cx);
                            window.push_notification(
                                Notification::success("Snapshot re-collected just now")
                                    .title("Refreshed"),
                                cx,
                            );
                        });
                    }
                }),
        );
        if self.tab == Tab::Overview {
            let report = overview_report(&self.snapshot, serial_number());
            actions = actions.child(
                Button::new("header-copy")
                    .primary()
                    .label("Copy details")
                    .icon(IconName::Copy)
                    .on_click(move |_, window, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(report.clone()));
                        window.push_notification(
                            Notification::success("Overview details copied to clipboard")
                                .title("Copied details"),
                            cx,
                        );
                    }),
            );
        }
        div()
            .flex()
            .items_center()
            .gap(px(16.))
            .child(header_tile(self.tab.icon(), cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .text_2xl()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(self.tab.label()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.subtitle()),
                    )
                    .child(div().flex().gap(px(6.)).mt(px(10.)).children(stats)),
            )
            .child(div().flex_1())
            .child(actions)
    }

    fn sidebar(&self, view: &Entity<Self>, cx: &mut Context<Self>) -> Sidebar<NavItem> {
        let mut rows = vec![
            NavItem::Tab {
                tab: Tab::Overview,
                active: self.tab == Tab::Overview,
                view: view.clone(),
            },
            NavItem::Group("Details"),
        ];
        for tab in TABS.iter().skip(1) {
            rows.push(NavItem::Tab {
                tab: *tab,
                active: self.tab == *tab,
                view: view.clone(),
            });
        }
        let about = Button::new("footer-about")
            .ghost()
            .small()
            .label("About")
            .icon(IconName::Info)
            .on_click(|_, window, cx| dialogs::show_about(window, cx));
        let theme_icon = if theme::is_dark(cx) {
            IconName::Moon
        } else {
            IconName::Sun
        };
        let theme_toggle = Button::new("footer-theme")
            .ghost()
            .small()
            .icon(theme_icon)
            .tooltip("Toggle theme")
            .on_click(|_, window, cx| theme::toggle_mode(window, cx));
        Sidebar::new("sidebar")
            .collapsible(false)
            .w(px(SIDEBAR_WIDTH))
            .header(brand(cx.theme().foreground, cx.theme().background))
            .children(rows)
            .footer(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .pt(px(6.))
                    .mt(px(6.))
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(div().flex_1().flex().justify_start().child(about))
                    .child(div().w(px(36.)).flex().justify_center().child(theme_toggle)),
            )
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .text_base()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(self.toolbar_title()),
            )
            .child(div().flex_1())
    }

    fn body(&self, cx: &mut Context<Self>) -> Div {
        let content: Div = match self.tab {
            Tab::Overview => {
                let toggle = match serial_number() {
                    Some(_) => {
                        let label = if self.serial_shown { "Hide" } else { "Show" };
                        let view = cx.entity();
                        Some(
                            Button::new("serial-toggle")
                                .outline()
                                .small()
                                .label(label)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| this.toggle_serial(cx));
                                })
                                .into_any_element(),
                        )
                    }
                    None => None,
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .children(views::overview(
                        &self.snapshot,
                        serial_number(),
                        self.serial_shown,
                        toggle,
                        cx,
                    ))
            }
            Tab::Processor => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::processor(&self.snapshot, cx)),
            Tab::Memory => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::memory(&self.snapshot, cx)),
            Tab::Network => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::network(&self.snapshot, cx)),
            _ => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(views::rebuilding(self.tab, cx)),
        };
        div().flex().flex_col().flex_1().min_h_0().child(content)
    }
}

/// The Overview copy target: plain facts, one per line.
fn overview_report(snapshot: &SystemSnapshot, serial: Option<String>) -> String {
    let mut lines = vec![
        format!("Distribution: {}", snapshot.os.distro),
        format!("Kernel: {}", snapshot.os.kernel_version),
        format!("Hostname: {}", snapshot.os.hostname),
        format!("Uptime: {}", snapshot.os.uptime),
        format!(
            "Processor: {} ({} threads)",
            snapshot.cpu.model, snapshot.cpu.logical_threads
        ),
        format!(
            "Memory: {} used of {}",
            sysinfo_viewer::data::format_bytes(snapshot.memory.used_bytes),
            sysinfo_viewer::data::format_bytes(snapshot.memory.total_bytes)
        ),
    ];
    if let Some(number) = serial {
        lines.push(format!("Serial number: {number}"));
    }
    lines.join("\n")
}

/// `lspci` absence is the one external-tool state with UI consequences
/// (no GPU rows), so it is detected on every refresh for the notice.
fn lspci_missing() -> bool {
    std::process::Command::new("lspci")
        .arg("--version")
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(true)
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .key_context(KEY_CONTEXT)
            .child(self.sidebar(&view, cx))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .p(px(24.))
                    .child(self.header(&view, window, cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .mt(px(18.))
                            .child(self.toolbar(cx))
                            .child(
                                div()
                                    .id("content")
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .overflow_y_scrollbar()
                                    .child(self.body(cx)),
                            ),
                    ),
            )
    }
}
