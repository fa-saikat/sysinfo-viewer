//! Rendering. One file, like the welcome app's `main.rs`, since the whole
//! UI is small enough that splitting it further would mean more
//! navigating between files than actual complexity saved. Data
//! collection lives in `crate::data` and stays untouched by any of this.
//!
//! A few style methods here (`flex_wrap()`, `gpui::relative()`,
//! directional borders like `border_r_1()`/`border_l_2()`/`border_b_1()`,
//! `flex_shrink_0()`) aren't demonstrated anywhere in the welcome app's
//! `main.rs`, so they're unverified against your exact pinned gpui
//! version — everything else here (`.flex()`, `.gap()`, `.rounded()`,
//! `.border_1()`, `.bg()`, `.text_size()`, `.when()`, `.hover()`,
//! `.cursor_pointer()`, `.on_click()`) is used the same way the welcome
//! app already proved works. If any of the unverified ones don't exist
//! in your version, they should have same-named or very close
//! equivalents (gpui's style API is fairly Tailwind-shaped throughout).

use gpui::{div, img, prelude::*, px, rgb, rgba, Context, FontWeight, ObjectFit, Window};

use crate::colors;
use crate::data::{
    format_bytes, format_frequency_mhz, GpuInfo, InterfaceState, MemoryInfo, NetworkInterface,
    StorageDevice, SystemSnapshot, VramSize,
};
use crate::tab::{BackgroundAvailability, Tab, TABS};

// ---------------------------------------------------------------------
// Resolution-aware scale — identical approach to the welcome app: pick a
// rem size proportional to window width so the layout composes the same
// way at 1080p and 4K, then build everything out of `rems()`.
// ---------------------------------------------------------------------
const BASE_REM_PX: f32 = 16.0;
const BASE_DESIGN_WIDTH: f32 = 1920.0;

fn apply_responsive_scale(window: &mut Window) {
    let size = window.viewport_size();
    let width: f32 = size.width.into();
    let scale = (width / BASE_DESIGN_WIDTH).clamp(0.7, 2.0);
    window.set_rem_size(px(BASE_REM_PX * scale));
}

fn r(rems: f32) -> gpui::Rems {
    gpui::rems(rems)
}

// ---------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------
pub struct SysInfoApp {
    tab: Tab,
    snapshot: SystemSnapshot,
    background_availability: BackgroundAvailability,
}

impl SysInfoApp {
    pub fn new(background_availability: BackgroundAvailability) -> Self {
        Self {
            tab: Tab::Overview,
            snapshot: SystemSnapshot::collect(),
            background_availability,
        }
    }

    fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.snapshot = SystemSnapshot::collect();
        cx.notify();
    }

    // -- shared chrome ----------------------------------------------------

    /// Full-bleed background for the current tab. Falls back to a flat
    /// `colors::BACKGROUND_FALLBACK` fill — no image, no scrim — when
    /// that tab's art hasn't been dropped into `assets/backgrounds/` yet,
    /// so a missing file never shows up as a broken image.
    fn background(&self) -> impl IntoElement {
        if self.background_availability.has_image(self.tab) {
            div()
                .absolute()
                .inset_0()
                .child(
                    img(self.tab.background_path())
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .child(div().absolute().inset_0().bg(rgba(colors::SCRIM)))
        } else {
            // No art for this tab yet — flat fill, no image element, no
            // scrim. Nothing here ever asks GPUI to load a path that
            // isn't on disk.
            div().absolute().inset_0().bg(rgb(colors::BACKGROUND_FALLBACK))
        }
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(r(11.5))
            .flex_shrink_0()
            .h_full()
            .p(r(0.75))
            .gap(r(0.15))
            .bg(rgba(colors::SIDEBAR_BG))
            .border_r_1()
            .border_color(rgba(colors::CARD_BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(0.5))
                    .px(r(0.4))
                    .pb(r(0.9))
                    .child(
                        div()
                            .text_size(r(1.1))
                            .text_color(rgb(colors::ACCENT))
                            .child("◆"),
                    )
                    .child(
                        div()
                            .text_size(r(0.9))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(colors::TEXT_PRIMARY))
                            .child("SysInfo"),
                    ),
            )
            .children(TABS.iter().map(|tab| self.sidebar_item(*tab, cx)))
    }

    fn sidebar_item(&self, tab: Tab, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.tab == tab;

        div()
            .id(tab.label())
            .flex()
            .items_center()
            .gap(r(0.55))
            .px(r(0.65))
            .h(r(2.1))
            .rounded(r(0.4))
            .cursor_pointer()
            .when(active, |d| {
                d.bg(rgba(0x8b5cf62e))
                    .border_l_2()
                    .border_color(rgb(colors::ACCENT))
            })
            .child(
                div()
                    .text_size(r(0.85))
                    .text_color(if active {
                        rgb(colors::ACCENT)
                    } else {
                        rgb(colors::TEXT_MUTED)
                    })
                    .child(tab.icon_glyph()),
            )
            .child(
                div()
                    .text_size(r(0.72))
                    .text_color(if active {
                        rgb(colors::TEXT_PRIMARY)
                    } else {
                        rgb(colors::TEXT_SECONDARY)
                    })
                    .child(tab.label()),
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.set_tab(tab, cx);
            }))
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .pb(r(0.75))
            .mb(r(0.75))
            .border_b_1()
            .border_color(rgba(colors::CARD_BORDER))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(r(0.95))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(colors::TEXT_PRIMARY))
                            .child(if self.tab == Tab::Overview {
                                "System Information".to_string()
                            } else {
                                self.tab.label().to_string()
                            }),
                    )
                    .child(
                        div()
                            .text_size(r(0.7))
                            .text_color(rgb(colors::TEXT_SECONDARY))
                            .child(if self.tab == Tab::Overview {
                                format!(
                                    "{} · {}",
                                    self.snapshot.os.hostname, self.snapshot.os.distro
                                )
                            } else {
                                self.tab.subtitle().to_string()
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(0.6))
                    .child(
                        div()
                            .text_size(r(0.65))
                            .text_color(rgb(colors::TEXT_MUTED))
                            .child(format!("Updated {}", self.snapshot.age_label())),
                    )
                    .child(
                        div()
                            .id("refresh")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(r(1.7))
                            .rounded(r(0.35))
                            .border_1()
                            .border_color(rgba(colors::CARD_BORDER))
                            .bg(rgba(colors::CARD_BG))
                            .text_size(r(0.8))
                            .text_color(rgb(colors::TEXT_SECONDARY))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(colors::TEXT_PRIMARY)))
                            .child("↻")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.refresh(cx);
                            })),
                    ),
            )
    }

    // -- shared card / row primitives --------------------------------------

    fn card(&self) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .p(r(0.85))
            .gap(r(0.5))
            .rounded(r(0.6))
            .bg(rgba(colors::CARD_BG))
            .border_1()
            .border_color(rgba(colors::CARD_BORDER))
    }

    /// Same fill/border/padding as `card()`, laid out as a row instead of
    /// a column — kept as a separate constructor rather than a
    /// `.flex_row()` override so both stay unambiguous about their type.
    fn card_row(&self) -> gpui::Div {
        div()
            .flex()
            .flex_row()
            .p(r(0.85))
            .gap(r(0.85))
            .rounded(r(0.6))
            .bg(rgba(colors::CARD_BG))
            .border_1()
            .border_color(rgba(colors::CARD_BORDER))
    }

    /// Wraps `items` into an n-column layout with `flex_wrap()` rather
    /// than a CSS-grid API — the welcome app never used one, so this
    /// stays on primitives already proven to work in this codebase.
    /// Width fractions are trimmed slightly below `1/cols` to leave room
    /// for the gap between cells without forcing an extra wrap.
    fn grid(&self, cols: usize, items: Vec<gpui::AnyElement>) -> impl IntoElement {
        let width_fraction = match cols {
            2 => 0.485,
            3 => 0.31,
            4 => 0.225,
            n => 1.0 / n as f32,
        };

        div().flex().flex_wrap().gap(r(0.75)).children(
            items
                .into_iter()
                .map(|el| div().w(gpui::relative(width_fraction)).child(el)),
        )
    }

    fn card_label(&self, label: &'static str) -> impl IntoElement {
        div()
            .text_size(r(0.62))
            .text_color(rgb(colors::TEXT_MUTED))
            .child(label)
    }

    fn stat_row(&self, label: impl Into<String>, value: impl Into<String>) -> impl IntoElement {
        div()
            .flex()
            .justify_between()
            .gap(r(0.75))
            .child(
                div()
                    .text_size(r(0.68))
                    .text_color(rgb(colors::TEXT_SECONDARY))
                    .child(label.into()),
            )
            .child(
                div()
                    .text_size(r(0.68))
                    .text_color(rgb(colors::TEXT_PRIMARY))
                    .child(value.into()),
            )
    }

    fn usage_bar(&self, fraction: f32) -> impl IntoElement {
        div()
            .h(r(0.35))
            .rounded(r(0.2))
            .bg(rgba(0xffffff14))
            .child(
                div()
                    .h_full()
                    .rounded(r(0.2))
                    .bg(rgb(colors::ACCENT))
                    .w(gpui::relative(fraction.clamp(0.0, 1.0))),
            )
    }

    fn badge(&self, label: &'static str, positive: bool) -> impl IntoElement {
        div()
            .px(r(0.4))
            .py(r(0.08))
            .rounded(r(0.75))
            .text_size(r(0.58))
            .when(positive, |d| {
                d.bg(rgb(colors::SUCCESS_BG))
                    .text_color(rgb(colors::SUCCESS))
                    .border_1()
                    .border_color(rgb(colors::SUCCESS))
            })
            .when(!positive, |d| {
                d.bg(rgba(0xffffff0f))
                    .text_color(rgb(colors::TEXT_MUTED))
                    .border_1()
                    .border_color(rgba(colors::CARD_BORDER))
            })
            .child(label)
    }

    // -- tabs ---------------------------------------------------------------

    fn render_overview(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let os = &self.snapshot.os;
        let cpu = &self.snapshot.cpu;
        let mem = &self.snapshot.memory;

        div()
            .flex()
            .flex_col()
            .gap(r(0.75))
            .child(self.grid(
                2,
                vec![
                    self.card()
                        .child(self.card_label("OS & KERNEL"))
                        .child(self.stat_row("Distribution", os.distro.clone()))
                        .child(self.stat_row("Kernel", os.kernel_version.clone()))
                        .child(self.stat_row("Hostname", os.hostname.clone()))
                        .child(self.stat_row("Uptime", os.uptime.clone()))
                        .into_any_element(),
                    self.card()
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .items_center()
                                .child(self.card_label("PROCESSOR"))
                                .child(self.badge(
                                    cpu.virtualization.label(),
                                    cpu.virtualization.is_enabled(),
                                )),
                        )
                        .child(
                            div()
                                .text_size(r(0.72))
                                .text_color(rgb(colors::TEXT_PRIMARY))
                                .child(cpu.model.clone()),
                        )
                        .child(self.stat_row(
                            "Cores / threads",
                            format!("{} / {}", cpu.physical_cores, cpu.logical_threads),
                        ))
                        .child(self.stat_row(
                            "Max frequency",
                            cpu.max_frequency_mhz
                                .map(format_frequency_mhz)
                                .unwrap_or_else(|| "Unknown".to_string()),
                        ))
                        .into_any_element(),
                    self.memory_summary_card(mem).into_any_element(),
                    self.graphics_summary_card().into_any_element(),
                ],
            ))
            .child(self.storage_card(&self.snapshot.storage))
            .child(self.network_card(&self.snapshot.network))
    }

    fn memory_summary_card(&self, mem: &MemoryInfo) -> impl IntoElement {
        self.card()
            .child(self.card_label("MEMORY"))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(r(0.68))
                            .text_color(rgb(colors::TEXT_SECONDARY))
                            .child(format!("{} used", format_bytes(mem.used_bytes))),
                    )
                    .child(
                        div()
                            .text_size(r(0.68))
                            .text_color(rgb(colors::TEXT_MUTED))
                            .child(format!("of {}", format_bytes(mem.total_bytes))),
                    ),
            )
            .child(self.usage_bar(mem.used_fraction()))
            .child(self.stat_row("Available", format_bytes(mem.available_bytes)))
    }

    fn graphics_summary_card(&self) -> impl IntoElement {
        let card = self.card().child(self.card_label("GRAPHICS"));

        match self.snapshot.gpus.first() {
            Some(gpu) => card
                .child(
                    div()
                        .text_size(r(0.72))
                        .text_color(rgb(colors::TEXT_PRIMARY))
                        .child(gpu.model.clone()),
                )
                .child(self.stat_row("VRAM", vram_label(&gpu.vram)))
                .child(self.stat_row("Driver", gpu.driver.clone())),
            None => card.child(
                div()
                    .text_size(r(0.68))
                    .text_color(rgb(colors::TEXT_MUTED))
                    .child("No GPU detected"),
            ),
        }
    }

    fn render_processor(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let cpu = &self.snapshot.cpu;
        let max_freq = cpu.max_frequency_mhz;

        div()
            .flex()
            .flex_col()
            .gap(r(0.75))
            .child(
                self.card_row()
                    .items_center()
                    .child(
                        div()
                            .size(r(2.2))
                            .rounded(r(0.5))
                            .bg(rgba(0x8b5cf62e))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(r(1.1))
                            .text_color(rgb(colors::ACCENT))
                            .child("◧"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(r(0.8))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(colors::TEXT_PRIMARY))
                                    .child(cpu.model.clone()),
                            )
                            .child(
                                div()
                                    .text_size(r(0.62))
                                    .text_color(rgb(colors::TEXT_MUTED))
                                    .child(format!(
                                        "{} · {} cores / {} threads",
                                        cpu.architecture, cpu.physical_cores, cpu.logical_threads
                                    )),
                            ),
                    )
                    .child(self.badge(
                        cpu.virtualization.label(),
                        cpu.virtualization.is_enabled(),
                    )),
            )
            .child(self.grid(
                4,
                vec![
                    self.stat_card("ARCHITECTURE", cpu.architecture.clone())
                        .into_any_element(),
                    self.stat_card(
                        "CORES / THREADS",
                        format!("{} / {}", cpu.physical_cores, cpu.logical_threads),
                    )
                    .into_any_element(),
                    self.stat_card(
                        "MAX FREQUENCY",
                        max_freq
                            .map(format_frequency_mhz)
                            .unwrap_or_else(|| "Unknown".to_string()),
                    )
                    .into_any_element(),
                    self.stat_card("VIRTUALIZATION", cpu.virtualization.label().to_string())
                        .into_any_element(),
                ],
            ))
            .child(
                self.card()
                    .child(self.card_label("PER-CORE FREQUENCY"))
                    .child(self.grid(
                        4,
                        cpu.per_core_frequency_mhz
                            .iter()
                            .enumerate()
                            .map(|(i, freq)| self.core_freq_cell(i, *freq, max_freq).into_any_element())
                            .collect(),
                    )),
            )
    }

    fn stat_card(&self, label: &'static str, value: String) -> impl IntoElement {
        self.card()
            .gap(r(0.3))
            .child(self.card_label(label))
            .child(
                div()
                    .text_size(r(0.85))
                    .text_color(rgb(colors::TEXT_PRIMARY))
                    .child(value),
            )
    }

    fn core_freq_cell(&self, index: usize, freq_mhz: u64, max_freq_mhz: Option<u64>) -> impl IntoElement {
        let fraction = max_freq_mhz
            .filter(|m| *m > 0)
            .map(|m| freq_mhz as f32 / m as f32)
            .unwrap_or(0.0);

        div()
            .flex()
            .flex_col()
            .gap(r(0.2))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(r(0.6))
                            .text_color(rgb(colors::TEXT_SECONDARY))
                            .child(format!("Core {index}")),
                    )
                    .child(
                        div()
                            .text_size(r(0.6))
                            .text_color(rgb(colors::TEXT_PRIMARY))
                            .child(format_frequency_mhz(freq_mhz)),
                    ),
            )
            .child(self.usage_bar(fraction))
    }

    fn render_memory(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let mem = &self.snapshot.memory;

        div()
            .flex()
            .flex_col()
            .gap(r(0.75))
            .child(
                self.card()
                    .child(self.card_label("RAM"))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_baseline()
                            .child(
                                div()
                                    .text_size(r(1.0))
                                    .text_color(rgb(colors::TEXT_PRIMARY))
                                    .child(format_bytes(mem.used_bytes)),
                            )
                            .child(
                                div()
                                    .text_size(r(0.68))
                                    .text_color(rgb(colors::TEXT_MUTED))
                                    .child(format!("of {} used", format_bytes(mem.total_bytes))),
                            ),
                    )
                    .child(self.usage_bar(mem.used_fraction())),
            )
            .child(self.grid(
                3,
                vec![
                    self.stat_card("TOTAL", format_bytes(mem.total_bytes)).into_any_element(),
                    self.stat_card("USED", format_bytes(mem.used_bytes)).into_any_element(),
                    self.stat_card("AVAILABLE", format_bytes(mem.available_bytes))
                        .into_any_element(),
                ],
            ))
            .child(if mem.has_swap() {
                self.card()
                    .child(self.card_label("SWAP"))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(r(0.68))
                                    .text_color(rgb(colors::TEXT_SECONDARY))
                                    .child(format_bytes(mem.swap_used_bytes)),
                            )
                            .child(
                                div()
                                    .text_size(r(0.68))
                                    .text_color(rgb(colors::TEXT_MUTED))
                                    .child(format!("of {}", format_bytes(mem.swap_total_bytes))),
                            ),
                    )
                    .child(self.usage_bar(if mem.swap_total_bytes == 0 {
                        0.0
                    } else {
                        mem.swap_used_bytes as f32 / mem.swap_total_bytes as f32
                    }))
                    .into_any_element()
            } else {
                self.card()
                    .child(self.card_label("SWAP"))
                    .child(
                        div()
                            .text_size(r(0.68))
                            .text_color(rgb(colors::TEXT_MUTED))
                            .child("No swap configured"),
                    )
                    .into_any_element()
            })
    }

    fn render_network(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(r(0.6))
            .children(self.snapshot.network.iter().map(|iface| self.network_row(iface, true)))
    }

    fn network_card(&self, interfaces: &[NetworkInterface]) -> impl IntoElement {
        self.card()
            .child(self.card_label("CONNECTIVITY"))
            .children(interfaces.iter().map(|iface| self.network_row(iface, false)))
    }

    fn network_row(&self, iface: &NetworkInterface, roomy: bool) -> impl IntoElement {
        let up = iface.state == InterfaceState::Up;

        let row = div()
            .flex()
            .items_center()
            .gap(r(0.6))
            .when(roomy, |d| {
                d.p(r(0.65))
                    .rounded(r(0.4))
                    .bg(rgba(colors::CARD_BG))
                    .border_1()
                    .border_color(rgba(colors::CARD_BORDER))
            })
            .child(
                div()
                    .size(r(0.4))
                    .rounded(r(1.0))
                    .bg(if up {
                        rgb(colors::SUCCESS)
                    } else {
                        rgb(colors::TEXT_MUTED)
                    }),
            )
            .child(
                div()
                    .w(r(4.5))
                    .flex_shrink_0()
                    .text_size(r(0.68))
                    .text_color(rgb(colors::TEXT_PRIMARY))
                    .child(iface.name.clone()),
            )
            .child(self.badge(iface.state.label(), up))
            .child(
                div()
                    .text_size(r(0.62))
                    .text_color(rgb(colors::TEXT_MUTED))
                    .child(iface.mac_address.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_end()
                    .text_size(r(0.62))
                    .text_color(rgb(colors::TEXT_SECONDARY))
                    .child(iface.ip_address.clone().unwrap_or_else(|| "—".to_string())),
            );

        row
    }

    fn render_storage(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(r(0.75))
            .children(self.snapshot.storage.iter().map(|device| self.storage_row(device, true)))
    }

    fn storage_card(&self, devices: &[StorageDevice]) -> impl IntoElement {
        self.card()
            .child(self.card_label("STORAGE"))
            .children(devices.iter().map(|device| self.storage_row(device, false)))
    }

    fn storage_row(&self, device: &StorageDevice, roomy: bool) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(r(0.3))
            .when(roomy, |d| {
                d.p(r(0.7))
                    .rounded(r(0.4))
                    .bg(rgba(colors::CARD_BG))
                    .border_1()
                    .border_color(rgba(colors::CARD_BORDER))
            })
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .gap(r(0.35))
                            .text_size(r(0.68))
                            .child(
                                div()
                                    .text_color(rgb(colors::TEXT_PRIMARY))
                                    .child(device.device_name.clone()),
                            )
                            .child(
                                div()
                                    .text_color(rgb(colors::TEXT_MUTED))
                                    .child(format!("→ {}", device.mount_point)),
                            )
                            .when(roomy, |d| {
                                d.child(
                                    div()
                                        .text_color(rgb(colors::TEXT_MUTED))
                                        .child(format!("({})", device.filesystem)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(r(0.62))
                            .text_color(rgb(colors::TEXT_SECONDARY))
                            .child(format!(
                                "{} / {}",
                                format_bytes(device.used_bytes()),
                                format_bytes(device.total_bytes)
                            )),
                    ),
            )
            .child(self.usage_bar(device.used_fraction()))
    }

    fn render_graphics(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.snapshot.gpus.is_empty() {
            return div()
                .child(
                    self.card().child(
                        div()
                            .text_size(r(0.7))
                            .text_color(rgb(colors::TEXT_MUTED))
                            .child("No GPU detected — is `lspci` installed?"),
                    ),
                )
                .into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .gap(r(0.75))
            .children(self.snapshot.gpus.iter().map(|gpu| self.gpu_card(gpu)))
            .into_any_element()
    }

    fn gpu_card(&self, gpu: &GpuInfo) -> impl IntoElement {
        self.card_row()
            .items_center()
            .child(
                div()
                    .size(r(2.2))
                    .rounded(r(0.5))
                    .bg(rgba(0x8b5cf62e))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(r(1.1))
                    .text_color(rgb(colors::ACCENT))
                    .child("◈"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap(r(0.15))
                    .child(
                        div()
                            .text_size(r(0.8))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(colors::TEXT_PRIMARY))
                            .child(gpu.model.clone()),
                    )
                    .child(
                        div()
                            .text_size(r(0.62))
                            .text_color(rgb(colors::TEXT_MUTED))
                            .child(format!(
                                "VRAM {} · Driver {} · {}",
                                vram_label(&gpu.vram),
                                gpu.driver,
                                gpu.pci_bus_id
                            )),
                    ),
            )
    }
}

fn vram_label(vram: &VramSize) -> String {
    match vram {
        VramSize::Dedicated(bytes) => format_bytes(*bytes),
        VramSize::Shared => "Shared".to_string(),
        VramSize::Unknown => "Unknown".to_string(),
    }
}

impl Render for SysInfoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        apply_responsive_scale(window);

        let content = match self.tab {
            Tab::Overview => self.render_overview(cx).into_any_element(),
            Tab::Processor => self.render_processor(cx).into_any_element(),
            Tab::Memory => self.render_memory(cx).into_any_element(),
            Tab::Network => self.render_network(cx).into_any_element(),
            Tab::Storage => self.render_storage(cx).into_any_element(),
            Tab::Graphics => self.render_graphics(cx).into_any_element(),
        };

        div()
            .relative()
            .flex()
            .size_full()
            .bg(rgb(colors::PANEL))
            .child(self.background())
            .child(self.sidebar(cx))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.0))
                    .p(r(1.0))
                    .child(self.header(cx))
                    // NOTE: the welcome app never needed scrolling (each
                    // slide fit one screen), so this doesn't wire up
                    // GPUI's scroll handle yet. If a tab's content grows
                    // past the window height on your hardware (e.g. many
                    // storage devices or network interfaces), add
                    // whatever scroll-container API your pinned gpui
                    // version exposes around this div.
                    .child(div().flex().flex_col().flex_1().child(content)),
            )
    }
}
