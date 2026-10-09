//! Tab bodies: description lists, tables and charts per tab.
//!
//! Each builder is pure layout over a collected snapshot. Search filtering
//! and the per-core chart arrive with their own tickets; until then every
//! tab renders its full honest content.

use gpui_kit::component::chart::LineChart;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle,
};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, white, AnyElement, App, Div, Styled};

use super::library::{fact_tag, group_title, warn_tag};
use super::sidebar::context_tile;
use crate::tab::Tab;
use sysinfo_viewer::data::{
    format_bytes, format_frequency_mhz, product_name, CpuInfo, SystemSnapshot,
};

/// One description-list group inside a rounded card.
fn group(title: &'static str, list: DescriptionList, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(group_title(title, None, cx))
        .child(
            div()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary.opacity(0.45))
                .border_1()
                .border_color(cx.theme().border.opacity(0.7))
                .child(list),
        )
}

fn list(rows: Vec<(&'static str, Div)>) -> DescriptionList {
    let mut dl = DescriptionList::new()
        .label_width(px(96.))
        .columns(1)
        .bordered(false);
    for (label, value) in rows {
        dl = dl.item(label, value.into_any_element(), 1);
    }
    dl
}

fn body(text: &str, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().foreground)
        .child(text.to_string())
}

fn mono(text: &str, cx: &App) -> Div {
    div()
        .text_sm()
        .font_family(cx.theme().mono_font_family.clone())
        .text_color(cx.theme().foreground)
        .child(text.to_string())
}

fn muted(text: &str, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text.to_string())
}

/// Overview: device facts plus software facts, with the serial toggle on
/// the same line as the serial itself.
pub fn overview(
    snapshot: &SystemSnapshot,
    serial: Option<String>,
    serial_shown: bool,
    serial_toggle: Option<AnyElement>,
    cx: &App,
) -> Vec<Div> {
    let cpu = &snapshot.cpu;
    let max_freq = cpu
        .max_frequency_mhz
        .map(format_frequency_mhz)
        .unwrap_or_else(|| "Unknown".to_string());
    let storage_total: u64 = snapshot.storage.iter().map(|d| d.total_bytes).sum();

    let mut device = vec![
        (
            "Model",
            body(&product_name().unwrap_or_else(|| "Unknown".to_string()), cx),
        ),
        (
            "Processor",
            body(
                &format!(
                    "{} · {} threads · {}",
                    cpu.model, cpu.logical_threads, max_freq
                ),
                cx,
            ),
        ),
        (
            "Memory",
            body(&format_bytes(snapshot.memory.total_bytes), cx),
        ),
        (
            "Graphics",
            body(
                &snapshot
                    .gpus
                    .first()
                    .map(|gpu| gpu.model.clone())
                    .unwrap_or_else(|| "No GPU detected".to_string()),
                cx,
            ),
        ),
        (
            "Storage",
            body(
                &if storage_total > 0 {
                    format_bytes(storage_total)
                } else {
                    "No mounted devices".to_string()
                },
                cx,
            ),
        ),
    ];
    if let Some(number) = serial {
        let shown = if serial_shown {
            number
        } else {
            "••••••••••".to_string()
        };
        let mut row = div().flex().items_center().gap(px(10.)).child(mono(&shown, cx));
        if let Some(toggle) = serial_toggle {
            row = row.child(toggle);
        }
        device.push(("Serial number", row));
    } else {
        device.push(("Serial number", muted("Unknown", cx)));
    }

    let software = vec![
        ("Distribution", body(&snapshot.os.distro, cx)),
        ("Kernel", mono(&snapshot.os.kernel_version, cx)),
        ("Architecture", body(&snapshot.cpu.architecture, cx)),
        ("Hostname", mono(&snapshot.os.hostname, cx)),
        ("Uptime", body(&snapshot.os.uptime, cx)),
    ];

    vec![
        group("Device", list(device), cx),
        group("Software", list(software), cx),
    ]
}

/// Processor: facts plus per-core frequency as a line chart (x is core
/// index, y is MHz, linear interpolation, dots, accent stroke, y-axis
/// with tick labels). An empty reading list renders honestly instead of
/// an empty chart.
pub fn processor(snapshot: &SystemSnapshot, cx: &App) -> Vec<Div> {
    let cpu = &snapshot.cpu;
    let virt = cpu.virtualization;
    let virt_value = div().child(if virt.is_enabled() {
        fact_tag(virt.label().to_string())
    } else {
        warn_tag(virt.label())
    });
    let mut details = vec![
        ("Model", body(&cpu.model, cx)),
        (
            "Cores / threads",
            body(&format!("{} / {}", cpu.physical_cores, cpu.logical_threads), cx),
        ),
        (
            "Max frequency",
            body(
                &cpu.max_frequency_mhz
                    .map(format_frequency_mhz)
                    .unwrap_or_else(|| "Unknown".to_string()),
                cx,
            ),
        ),
        ("Architecture", body(&cpu.architecture, cx)),
        ("Virtualization", virt_value),
    ];
    let mut groups = vec![group("Processor", list(details), cx)];

    if cpu.per_core_frequency_mhz.is_empty() {
        groups.push(group(
            "Per-core frequency",
            list(vec![("Readings", muted("No per-core readings", cx))]),
            cx,
        ));
    } else {
        let points = core_points(cpu);
        let chart = LineChart::new(points)
            .x(|point: &(String, f64)| point.0.clone())
            .y(|point: &(String, f64)| point.1)
            .linear()
            .dot()
            .stroke(cx.theme().primary)
            .y_axis(true)
            .y_tick_count(4)
            .y_tick_format(|value| format!("{:.1}", value / 1000.0))
            .x_tick_count(4);
        groups.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(group_title("Per-core frequency", Some("GHz".to_string()), cx))
                .child(
                    div()
                        .rounded(cx.theme().radius_lg)
                        .bg(cx.theme().secondary.opacity(0.45))
                        .border_1()
                        .border_color(cx.theme().border.opacity(0.7))
                        .p(px(16.))
                        .child(div().h(px(180.)).w_full().child(chart)),
                ),
        );
    }
    groups
}

/// Temporary honest state for tabs whose tickets have not landed yet.
pub fn rebuilding(tab: Tab, cx: &App) -> AnyElement {
    Empty::new()
        .header(
            EmptyHeader::new()
                .media(
                    EmptyMedia::new().child(context_tile(
                        tab.icon(),
                        56.0,
                        28.0,
                        cx.theme().primary,
                        white(),
                    )),
                )
                .title(EmptyTitle::new().child(format!("{} isn't rebuilt yet", tab.label())))
                .description(
                    EmptyDescription::new().child(
                        "This tab still runs on the old stack until its ticket lands.".to_string(),
                    ),
                ),
        )
        .content(EmptyContent::new())
        .into_any_element()
}

/// Stat tags for the page header, per tab.
pub fn header_stats(tab: Tab, snapshot: &SystemSnapshot, cx: &App) -> Vec<Tag> {
    let _ = cx;
    match tab {
        Tab::Overview => vec![
            fact_tag(format!("hostname {}", snapshot.os.hostname)),
            fact_tag(snapshot.os.kernel_version.clone()),
            fact_tag(format!("Uptime {}", snapshot.os.uptime)),
        ],
        Tab::Processor => vec![
            fact_tag(format!("{} threads", snapshot.cpu.logical_threads)),
            fact_tag(snapshot.cpu.architecture.clone()),
            fact_tag(snapshot.cpu.virtualization.label().to_string()),
        ],
        Tab::Memory => vec![
            fact_tag(format!(
                "{} of {}",
                format_bytes(snapshot.memory.used_bytes),
                format_bytes(snapshot.memory.total_bytes)
            )),
            fact_tag(format!(
                "{}% used",
                (snapshot.memory.used_fraction() * 100.0).round() as u32
            )),
        ],
        Tab::Network => vec![fact_tag(format!(
            "{} interfaces",
            snapshot.network.len()
        ))],
        Tab::Storage => {
            let total: u64 = snapshot.storage.iter().map(|d| d.total_bytes).sum();
            let used: u64 = snapshot.storage.iter().map(|d| d.used_bytes()).sum();
            vec![fact_tag(format!(
                "{} of {}",
                format_bytes(used),
                format_bytes(total)
            ))]
        }
        Tab::Graphics => match snapshot.gpus.first() {
            Some(gpu) => vec![
                fact_tag(gpu.model.clone()),
                fact_tag(format!("{} · {}", gpu.driver, vram_short(&gpu.vram))),
            ],
            None => vec![fact_tag("No GPU detected".to_string())],
        },
    }
}

fn vram_short(vram: &sysinfo_viewer::data::VramSize) -> String {
    use sysinfo_viewer::data::VramSize;
    match vram {
        VramSize::Dedicated(bytes) => format_bytes(*bytes),
        VramSize::Shared => "Shared".to_string(),
        VramSize::Unknown => "Unknown".to_string(),
    }
}

/// Per-core chart points in tab order for the Processor ticket.
#[allow(dead_code)]
pub fn core_points(cpu: &CpuInfo) -> Vec<(String, f64)> {
    cpu.per_core_frequency_mhz
        .iter()
        .enumerate()
        .map(|(index, mhz)| (format!("Core {index}"), *mhz as f64))
        .collect()
}
