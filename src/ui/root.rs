//! App shell: state, actions, page header, toolbar and render.
//!
//! The content column is header → notice → toolbar → tab body. Handlers
//! capture the view entity and update through it, which keeps every
//! callback working from `App`-only contexts such as sidebar rows.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::sidebar::Sidebar;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt, WindowExt};
use gpui_kit::prelude::*;
use gpui_kit::{
    div, px, ClipboardItem, Context, Div, Entity, FocusHandle, Focusable, FontWeight,
    Keystroke, Render, Styled, Subscription, Window,
};
use super::dialogs;
use super::library::{header_tile, notice_row, small_icon};
use super::{views, FocusSearch};
use sysinfo_viewer::search::{count_matches, tab_matches};
use super::sidebar::{brand, NavItem, SIDEBAR_WIDTH};
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
    search: Entity<InputState>,
    query: String,
    slash: Kbd,
    _search_subscription: Subscription,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, initial: Option<Tab>) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search details")
        });
        let _search_subscription = cx.subscribe(&search, |this: &mut Self, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.on_search_changed(cx);
            }
        });
        Self {
            focus: cx.focus_handle(),
            tab: initial.unwrap_or(Tab::Overview),
            snapshot: SystemSnapshot::collect(),
            serial_shown: false,
            lspci_missing: lspci_missing(),
            search,
            query: String::new(),
            slash: Kbd::new(Keystroke::parse("/").expect("slash keystroke parses")),
            _search_subscription,
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

    /// Mirrors the input into the query and auto-switches to the first
    /// tab with a match when the current tab has none.
    fn on_search_changed(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        if query == self.query {
            return;
        }
        self.query = query.clone();
        if !query.trim().is_empty() {
            let (visible, _) =
                count_matches(self.tab.slug(), &self.snapshot, self.serial_shown, &query);
            if visible == 0 {
                for tab in TABS {
                    if tab != self.tab
                        && tab_matches(tab.slug(), &self.snapshot, self.serial_shown, &query)
                    {
                        self.tab = tab;
                        break;
                    }
                }
            }
        }
        cx.notify();
    }

    fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.search.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
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
        let stats = views::header_stats(self.tab, &self.snapshot);
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
            let report = overview_report(&self.snapshot, self.serial_shown);
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
                            .font_weight(FontWeight::BOLD)
                            .text_color(cx.theme().foreground)
                            .child(self.tab.label()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.subtitle()),
                    )
                    .child(div().flex().gap(px(8.)).mt(px(12.)).children(stats)),
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
        let distro = self.snapshot.os.distro.clone();
        let about = Button::new("footer-about")
            .ghost()
            .small()
            .label("About")
            .icon(IconName::Info)
            .on_click(move |_, window, cx| dialogs::show_about(window, cx, distro.clone()));
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
            .header(brand(
                cx.theme().foreground,
                cx.theme().background,
                cx.theme().muted_foreground,
            ))
            .children(rows)
            .footer(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .pt(px(10.))
                    .mt(px(6.))
                    .border_t_1()
                    .border_color(cx.theme().sidebar_border)
                    .child(div().flex_1().flex().justify_start().child(about))
                    .child(div().w(px(36.)).flex().justify_center().child(theme_toggle)),
            )
    }
    fn toolbar(&self, cx: &mut Context<Self>) -> Div {
        let mut heading = div()
            .flex()
            .items_baseline()
            .gap(px(6.))
            .text_base()
            .font_semibold()
            .text_color(cx.theme().foreground)
            .child(self.toolbar_title());
        if !self.query.trim().is_empty() {
            let (visible, total) =
                count_matches(self.tab.slug(), &self.snapshot, self.serial_shown, &self.query);
            heading = heading.child(
                div()
                    .text_xs()
                    .font_normal()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{visible} of {total}")),
            );
        }
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(heading)
            .child(div().flex_1())
            .child(
                div().w(px(240.)).child(
                    Input::new(&self.search)
                        .prefix(small_icon(IconName::Search, cx))
                        .suffix(self.slash.clone())
                        .cleanable(true),
                ),
            )
    }

    fn notice(&self, view: &Entity<Self>, cx: &mut Context<Self>) -> Option<Div> {
        if !self.lspci_missing {
            return None;
        }
        let action = Button::new("notice-refresh")
            .primary()
            .small()
            .label("Refresh")
            .on_click({
                let view = view.clone();
                move |_, window, cx| {
                    view.update(cx, |this, cx| {
                        this.refresh(cx);
                        window.push_notification(
                            Notification::success("Tools re-detected just now").title("Refreshed"),
                            cx,
                        );
                    });
                }
            });
        Some(notice_row(
            IconName::TriangleAlert,
            cx.theme().warning,
            "Some details need lspci",
            "Install pciutils, then choose Refresh to detect GPUs.".to_string(),
            div().child(action),
            cx,
        ))
    }

    fn body(&self, cx: &mut Context<Self>) -> Div {
        let query = self.query.clone();
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
                        &query,
                        cx,
                    ))
            }
            Tab::Processor => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::processor(&self.snapshot, &query, cx)),
            Tab::Memory => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::memory(&self.snapshot, &query, cx)),
            Tab::Network => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::network(&self.snapshot, &query, cx)),
            Tab::Storage => div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .children(views::storage(&self.snapshot, &query, cx)),
            Tab::Graphics => {
                let action = {
                    let view = cx.entity();
                    Button::new("empty-refresh")
                        .primary()
                        .small()
                        .label("Refresh")
                        .on_click(move |_, window, cx| {
                            view.update(cx, |this, cx| {
                                this.refresh(cx);
                                window.push_notification(
                                    Notification::success("Tools re-detected just now")
                                        .title("Refreshed"),
                                    cx,
                                );
                            });
                        })
                        .into_any_element()
                };
                div().flex().flex_col().gap(px(20.)).children(
                    views::graphics(
                        &self.snapshot,
                        self.lspci_missing,
                        Some(action),
                        &query,
                        cx,
                    ),
                )
            }
        };
        div().flex().flex_col().flex_1().min_h_0().child(content)
    }
}

/// The Copy details target: exactly what the Overview page shows, one
/// `Label: value` line per row. The serial goes out as displayed —
/// masked unless revealed — because the button copies the page.
fn overview_report(snapshot: &SystemSnapshot, serial_shown: bool) -> String {
    use sysinfo_viewer::data::{
        format_bytes, format_frequency_mhz, product_name, serial_number,
    };
    let cpu = &snapshot.cpu;
    let max_freq = cpu
        .max_frequency_mhz
        .map(format_frequency_mhz)
        .unwrap_or_else(|| "Unknown".to_string());
    let storage_total: u64 = snapshot.storage.iter().map(|d| d.total_bytes).sum();
    let serial = match serial_number() {
        Some(number) if serial_shown => number,
        Some(_) => "••••••••••".to_string(),
        None => "Unknown".to_string(),
    };
    [
        (
            "Model",
            product_name().unwrap_or_else(|| "Unknown".to_string()),
        ),
        (
            "Processor",
            format!(
                "{} · {} threads · {}",
                cpu.model, cpu.logical_threads, max_freq
            ),
        ),
        ("Memory", format_bytes(snapshot.memory.total_bytes)),
        (
            "Graphics",
            snapshot
                .gpus
                .first()
                .map(|gpu| gpu.model.clone())
                .unwrap_or_else(|| "No GPU detected".to_string()),
        ),
        (
            "Storage",
            if storage_total > 0 {
                format_bytes(storage_total)
            } else {
                "No mounted devices".to_string()
            },
        ),
        ("Serial number", serial),
        ("Distribution", snapshot.os.distro.clone()),
        ("Kernel", snapshot.os.kernel_version.clone()),
        ("Architecture", snapshot.cpu.architecture.clone()),
        ("Hostname", snapshot.os.hostname.clone()),
        ("Uptime", snapshot.os.uptime.clone()),
    ]
    .into_iter()
    .map(|(label, value)| format!("{label}: {value}"))
    .collect::<Vec<_>>()
    .join("\n")
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
        let mut below_header = div().flex().flex_col().gap(px(18.)).mt(px(18.));
        if let Some(notice) = self.notice(&view, cx) {
            below_header = below_header.child(notice);
        }
        below_header = below_header
            .child(self.toolbar(cx))
            .child(
                div()
                    .id("content")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .child(self.body(cx)),
            );
        div()
            .flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .key_context(KEY_CONTEXT)
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                this.focus_search(window, cx);
            }))
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
                    .child(below_header),
            )
    }
}
