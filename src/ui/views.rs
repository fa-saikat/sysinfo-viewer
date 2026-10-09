//! Tab bodies: description lists, tables and charts per tab.
//!
//! Every builder takes the toolbar query and hides non-matching rows.
//! Row inventories for counting and auto-switch live in [`row_texts`]
//! and [`group_titles`]: keep them in sync with what the builders render,
//! matching on the same plain strings the rows are built from.

use gpui_kit::component::chart::LineChart;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle,
};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, white, AnyElement, App, Div, Styled};

use super::library::{fact_tag, group_title, warn_tag};
use sysinfo_viewer::search::{is_connected, matches_query, row_texts};
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
        .child(card(list.into_any_element(), cx))
}

fn card(content: AnyElement, cx: &App) -> Div {
    div()
        .rounded(cx.theme().radius_lg)
        .bg(cx.theme().secondary.opacity(0.45))
        .border_1()
        .border_color(cx.theme().border.opacity(0.7))
        .child(content)
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

/// Card wrapper for content richer than a plain description list.
fn card_group(title: &'static str, content: Div, cx: &App) -> Div {
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
                .p(px(16.))
                .child(content),
        )
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

/// Keeps rendered rows aligned with the search inventory: filters the
/// built rows by the same strings the toolbar counts.
fn filter_rows(
    rows: Vec<(&'static str, Div)>,
    texts: &[String],
    query: &str,
) -> Vec<(&'static str, Div)> {
    rows.into_iter()
        .zip(texts.iter())
        .filter(|(_, text)| matches_query(query, text))
        .map(|((label, element), _)| (label, element))
        .collect()
}

/// Overview: device facts plus software facts, with the serial toggle on
/// the same line as the serial itself.
pub fn overview(
    snapshot: &SystemSnapshot,
    serial: Option<String>,
    serial_shown: bool,
    serial_toggle: Option<AnyElement>,
    query: &str,
    cx: &App,
) -> Vec<Div> {
    let cpu = &snapshot.cpu;
    let max_freq = cpu
        .max_frequency_mhz
        .map(format_frequency_mhz)
        .unwrap_or_else(|| "Unknown".to_string());
    let storage_total: u64 = snapshot.storage.iter().map(|d| d.total_bytes).sum();
    let model = product_name().unwrap_or_else(|| "Unknown".to_string());
    let processor = format!("{} · {} threads · {}", cpu.model, cpu.logical_threads, max_freq);
    let memory = format_bytes(snapshot.memory.total_bytes);
    let graphics = snapshot
        .gpus
        .first()
        .map(|gpu| gpu.model.clone())
        .unwrap_or_else(|| "No GPU detected".to_string());
    let storage = if storage_total > 0 {
        format_bytes(storage_total)
    } else {
        "No mounted devices".to_string()
    };

    let mut device: Vec<(&str, Div)> = vec![
        ("Model", body(&model, cx)),
        ("Processor", body(&processor, cx)),
        ("Memory", body(&memory, cx)),
        ("Graphics", body(&graphics, cx)),
        ("Storage", body(&storage, cx)),
    ];
    let shown = serial
        .as_deref()
        .map(|number| {
            if serial_shown {
                number.to_string()
            } else {
                "••••••••••".to_string()
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());
    let mut serial_row = div()
        .flex()
        .items_center()
        .gap(px(10.))
        .child(mono(&shown, cx));
    if serial.is_some() {
        if let Some(toggle) = serial_toggle {
            serial_row = serial_row.child(toggle);
        }
    }
    device.push(("Serial number", serial_row));

    let software = vec![
        ("Distribution", body(&snapshot.os.distro, cx)),
        ("Kernel", mono(&snapshot.os.kernel_version, cx)),
        ("Architecture", body(&snapshot.cpu.architecture, cx)),
        ("Hostname", mono(&snapshot.os.hostname, cx)),
        ("Uptime", body(&snapshot.os.uptime, cx)),
    ];

    let device = filter_rows(device, &row_texts("overview", snapshot, serial_shown), query);
    let software = filter_rows(software, &software_texts(snapshot), query);
    vec![
        group("Device", list(device), cx),
        group("Software", list(software), cx),
    ]
}

fn software_texts(snapshot: &SystemSnapshot) -> Vec<String> {
    let all = row_texts("overview", snapshot, false);
    all[6..].to_vec()
}

/// Processor: facts plus per-core frequency as a line chart (x is core
/// index, y is MHz, linear interpolation, dots, accent stroke, y-axis
/// with tick labels). An empty reading list renders honestly instead of
/// an empty chart.
pub fn processor(snapshot: &SystemSnapshot, query: &str, cx: &App) -> Vec<Div> {
    let cpu = &snapshot.cpu;
    let virt = cpu.virtualization;
    let details = vec![
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
        (
            "Virtualization",
            div().child(if virt.is_enabled() {
                fact_tag(virt.label().to_string())
            } else {
                warn_tag(virt.label())
            }),
        ),
    ];
    let details = filter_rows(details, &processor_detail_texts(snapshot), query);
    let mut groups = vec![group("Processor", list(details), cx)];

    let chart_matches = matches_query(query, "Per-core frequency")
        || cpu
            .per_core_frequency_mhz
            .iter()
            .enumerate()
            .any(|(index, mhz)| {
                matches_query(query, &format!("Core {index} {mhz} MHz"))
            });
    if !cpu.per_core_frequency_mhz.is_empty() && (query.trim().is_empty() || chart_matches) {
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
    } else if cpu.per_core_frequency_mhz.is_empty() {
        groups.push(group(
            "Per-core frequency",
            list(vec![("Readings", muted("No per-core readings", cx))]),
            cx,
        ));
    }
    groups
}

fn processor_detail_texts(snapshot: &SystemSnapshot) -> Vec<String> {
    row_texts("processor", snapshot, false)[..5].to_vec()
}

/// Memory: headline figures with neutral progress bars, plus the
/// honest no-swap state instead of an empty row.
pub fn memory(snapshot: &SystemSnapshot, query: &str, cx: &App) -> Vec<Div> {
    let mem = &snapshot.memory;
    let ram_rows = vec![
        ("Total", body(&format_bytes(mem.total_bytes), cx)),
        ("Used", body(&format_bytes(mem.used_bytes), cx)),
        ("Available", body(&format_bytes(mem.available_bytes), cx)),
    ];
    let ram = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            div()
                .flex()
                .justify_between()
                .items_baseline()
                .child(
                    div()
                        .text_xl()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(format_bytes(mem.used_bytes)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("of {} used", format_bytes(mem.total_bytes))),
                ),
        )
        .child(super::library::progress_bar(
            "memory-ram",
            mem.used_fraction() * 100.0,
        ))
        .child(list(filter_rows(
            ram_rows,
            &row_texts("memory", snapshot, false)[..3].to_vec(),
            query,
        )));
    let swap = if mem.has_swap() {
        let swap_rows = vec![
            ("Total", body(&format_bytes(mem.swap_total_bytes), cx)),
            ("Used", body(&format_bytes(mem.swap_used_bytes), cx)),
        ];
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_baseline()
                    .child(
                        div()
                            .text_xl()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(format_bytes(mem.swap_used_bytes)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("of {} used", format_bytes(mem.swap_total_bytes))),
                    ),
            )
            .child(super::library::progress_bar(
                "memory-swap",
                if mem.swap_total_bytes == 0 {
                    0.0
                } else {
                    mem.swap_used_bytes as f32 / mem.swap_total_bytes as f32 * 100.0
                },
            ))
            .child(list(filter_rows(
                swap_rows,
                &row_texts("memory", snapshot, false)[3..].to_vec(),
                query,
            )))
    } else {
        div().child(list(vec![("Swap", muted("No swap configured", cx))]))
    };
    vec![
        card_group("Memory", ram, cx),
        card_group("Swap", swap, cx),
    ]
}

/// Network: general facts plus the interfaces table. Only collected
/// facts render — link type and Wi-Fi details have no collector, so
/// they stay out rather than being invented.
pub fn network(snapshot: &SystemSnapshot, query: &str, cx: &App) -> Vec<Div> {
    let general = vec![
        ("Hostname", mono(&snapshot.os.hostname, cx)),
        (
            "Status",
            div().child(if is_connected(snapshot) {
                Tag::success().small().child("Connected".to_string())
            } else {
                Tag::secondary().small().child("Offline".to_string())
            }),
        ),
    ];
    let general = filter_rows(general, &row_texts("network", snapshot, false)[..2].to_vec(), query);
    let mut groups = vec![group("General", list(general), cx)];

    let texts = row_texts("network", snapshot, false);
    if snapshot.network.is_empty() {
        groups.push(group(
            "Interfaces",
            list(vec![("Interfaces", muted("No interfaces found", cx))]),
            cx,
        ));
        return groups;
    }

    let mut rows = div().flex().flex_col().child(table_head(
        vec![
            ("Name".to_string(), Some(90.0)),
            ("IPv4".to_string(), None),
            ("MAC".to_string(), Some(150.0)),
            ("State".to_string(), Some(64.0)),
        ],
        cx,
    ));
    for (iface, text) in snapshot.network.iter().zip(texts[2..].iter()) {
        if matches_query(query, text) {
            rows = rows.child(interface_row(iface, cx));
        }
    }
    groups.push(
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(group_title(
                "Interfaces",
                Some(format!("{} interfaces", snapshot.network.len())),
                cx,
            ))
            .child(
                div()
                    .rounded(cx.theme().radius_lg)
                    .bg(cx.theme().secondary.opacity(0.45))
                    .border_1()
                    .border_color(cx.theme().border.opacity(0.7))
                    .overflow_hidden()
                    .child(rows),
            ),
    );
    groups
}

/// Table header row: 34 high, muted small labels on the header fill.
/// A `None` width flexes to fill the remaining space.
fn table_head(columns: Vec<(String, Option<f32>)>, cx: &App) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap(px(12.))
        .h(px(34.))
        .px(px(12.))
        .bg(cx.theme().table_head);
    for (label, width) in columns {
        let cell = div()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().muted_foreground)
            .child(label);
        row = match width {
            Some(w) => row.child(div().w(px(w)).flex_shrink_0().child(cell)),
            None => row.child(div().flex_1().min_w_0().child(cell)),
        };
    }
    row
}

fn interface_row(iface: &sysinfo_viewer::data::NetworkInterface, cx: &App) -> Div {
    use sysinfo_viewer::data::InterfaceState;
    let state = match iface.state {
        InterfaceState::Up => div().child(Tag::success().small().child("Up".to_string())),
        InterfaceState::Down => div().child(Tag::secondary().small().child("Down".to_string())),
        InterfaceState::Unknown => {
            div().child(Tag::secondary().small().child("Unknown".to_string()))
        }
    };
    let mac = if iface.mac_address.is_empty() {
        muted("—", cx)
    } else {
        mono(&iface.mac_address, cx)
    };
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .h(px(54.))
        .px(px(12.))
        .border_t_1()
        .border_color(cx.theme().border.opacity(0.7))
        .hover(|style| style.bg(cx.theme().table_hover))
        .child(
            div()
                .w(px(90.))
                .flex_shrink_0()
                .child(mono(&iface.name, cx)),
        )
        .child(div().flex_1().min_w_0().child(mono(
            iface.ip_address.as_deref().unwrap_or("—"),
            cx,
        )))
        .child(div().w(px(150.)).flex_shrink_0().child(mac))
        .child(div().w(px(64.)).flex_shrink_0().child(state))
}

/// Storage: devices and partitions as a dense table with a neutral
/// usage bar per device. Only collected facts render.
pub fn storage(snapshot: &SystemSnapshot, query: &str, cx: &App) -> Vec<Div> {
    if snapshot.storage.is_empty() {
        return vec![group(
            "Devices",
            list(vec![("Devices", muted("No mounted devices", cx))]),
            cx,
        )];
    }
    let texts = row_texts("storage", snapshot, false);
    let mut rows = div().flex().flex_col().child(table_head(
        vec![
            ("Mount".to_string(), None),
            ("Used / Size".to_string(), Some(150.0)),
        ],
        cx,
    ));
    for (device, text) in snapshot.storage.iter().zip(texts.iter()) {
        if matches_query(query, text) {
            rows = rows.child(storage_row(device, cx));
        }
    }
    vec![div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(group_title(
            "Devices",
            Some(format!(
                "{} {}",
                snapshot.storage.len(),
                if snapshot.storage.len() == 1 {
                    "device"
                } else {
                    "devices"
                }
            )),
            cx,
        ))
        .child(
            div()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().secondary.opacity(0.45))
                .border_1()
                .border_color(cx.theme().border.opacity(0.7))
                .overflow_hidden()
                .child(rows),
        )]
}

fn storage_row(device: &sysinfo_viewer::data::StorageDevice, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .px(px(12.))
        .py(px(10.))
        .border_t_1()
        .border_color(cx.theme().border.opacity(0.7))
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(8.))
                .child(div().flex_1().min_w_0().child(mono(&device.mount_point, cx)))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{} · {}", device.device_name, device.filesystem)),
                )
                .child(
                    div()
                        .w(px(150.))
                        .flex_shrink_0()
                        .text_right()
                        .child(mono(
                            &format!(
                                "{} / {}",
                                format_bytes(device.used_bytes()),
                                format_bytes(device.total_bytes)
                            ),
                            cx,
                        )),
                ),
        )
        .child(super::library::progress_bar(
            format!("storage-{}", device.mount_point),
            device.used_fraction() * 100.0,
        ))
}

/// Graphics: one card per GPU with the measured VRAM amount, or an
/// honest empty state naming the missing tool with a recovery action.
pub fn graphics(
    snapshot: &SystemSnapshot,
    lspci_missing: bool,
    empty_action: Option<AnyElement>,
    query: &str,
    cx: &App,
) -> Vec<Div> {
    if snapshot.gpus.is_empty() {
        let mut empty = Empty::new().header(
            EmptyHeader::new()
                .media(EmptyMedia::new().child(context_tile(
                    Tab::Graphics.icon(),
                    56.0,
                    28.0,
                    cx.theme().primary,
                    white(),
                )))
                .title(EmptyTitle::new().child("No GPU details yet".to_string()))
                .description(EmptyDescription::new().child(
                    if lspci_missing {
                        "lspci is not installed, so this tab cannot list GPUs. Install pciutils, then choose Refresh.".to_string()
                    } else {
                        "No GPUs were detected on this system.".to_string()
                    },
                )),
        );
        if let Some(action) = empty_action {
            empty = empty.content(EmptyContent::new().child(action));
        } else {
            empty = empty.content(EmptyContent::new());
        }
        return vec![div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(group_title("Detected GPUs", None, cx))
            .child(empty.into_any_element())];
    }
    let total = snapshot.memory.total_bytes;
    let texts = row_texts("graphics", snapshot, false);
    let per_gpu = 3;
    vec![div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(group_title(
            "Detected GPUs",
            Some(format!(
                "{} {}",
                snapshot.gpus.len(),
                if snapshot.gpus.len() == 1 { "GPU" } else { "GPUs" }
            )),
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .children(
                    snapshot
                        .gpus
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| {
                            texts[index * per_gpu..(index + 1) * per_gpu]
                                .iter()
                                .any(|text| matches_query(query, text))
                        })
                        .map(|(_, gpu)| {
                            div()
                                .rounded(cx.theme().radius_lg)
                                .bg(cx.theme().secondary.opacity(0.45))
                                .border_1()
                                .border_color(cx.theme().border.opacity(0.7))
                                .child(list(
                                    vec![
                                        ("Model", body(&gpu.model, cx)),
                                        (
                                            "VRAM",
                                            body(
                                                &sysinfo_viewer::data::vram_label(&gpu.vram, total),
                                                cx,
                                            ),
                                        ),
                                        ("Driver", mono(&gpu.driver, cx)),
                                    ],
                                ))
                        }),
                )
                .into_any_element(),
        )]
}

/// Per-core chart points in tab order for the Processor chart.
pub fn core_points(cpu: &CpuInfo) -> Vec<(String, f64)> {
    cpu.per_core_frequency_mhz
        .iter()
        .enumerate()
        .map(|(index, mhz)| (format!("Core {index}"), *mhz as f64))
        .collect()
}

/// Stat tags for the page header, per tab.
pub fn header_stats(tab: Tab, snapshot: &SystemSnapshot) -> Vec<Tag> {
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
                fact_tag(format!(
                    "{} · {}",
                    gpu.driver,
                    sysinfo_viewer::data::vram_label(&gpu.vram, snapshot.memory.total_bytes)
                )),
            ],
            None => vec![fact_tag("No GPU detected".to_string())],
        },
    }
}
