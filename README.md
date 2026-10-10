# System Information Viewer

A GPUI dashboard for Linux desktop, built on the Shelf design system
(`design/DESIGN_SYSTEM.md`) shared with the ROM Manager: same sidebar,
page header, toolbar search, description lists, tables, chart, notices,
dialogs and notifications, with one brand accent (`src/theme.rs`).

## Layout

```
src/
  main.rs   — kit bootstrap (theme, accent, key bindings, window)
  theme.rs  — brand accent on top of the kit theme + light/dark toggle
  assets.rs — Lucide icon registry + app asset source
  tab.rs    — Tab enum, labels, icons, launch-switch slugs
  search.rs — search inventory (row texts, counts, auto-switch)
  app state + rendering:
  ui/
    mod.rs      — modules + FocusSearch action
    root.rs     — RootView state, header, toolbar, notices, render
    sidebar.rs  — brand mark, context rows, accent tiles
    library.rs  — shared primitives (header tile, tags, notices, bars)
    views.rs    — the six tab bodies
    dialogs.rs  — About dialog
  data/       — collectors + formatters, zero UI dependencies
    mod.rs      — SystemSnapshot::collect(), shared formatters
    os.rs       — distro, kernel, hostname, uptime, DMI serial/name
    cpu.rs      — model, arch, cores/threads, max frequency, virtualization
    memory.rs   — RAM + swap
    network.rs  — interfaces: state, MAC, IP
    storage.rs  — mounted devices + usage
    gpu.rs      — model, VRAM, driver (the trickiest collector — see below)
tests/
  harness.rs  — full-stack proving loop, see docs/testing.md
```

Data collection (`src/data/`) has zero UI dependencies. The
integration harness collects from fixture roots through the same
formatters and snapshot shape the UI renders, in one `cargo test`
process. See `docs/testing.md` for the loop (fixtures, `SYSINFO_*`
switches, screenshot matrix).

## Build

```sh
cargo build --release
```

The suite pins one GPUI snapshot (`gpui-kit = "=0.7.1"` in
`Cargo.toml`); bump that one line to move every suite app together.
System backends (X11/Wayland) come with the kit's defaults.

Harness launch switches (dark Overview unless set):

| Variable | Effect |
|---|---|
| `SYSINFO_TAB=processor` | Open on that tab |
| `SYSINFO_THEME=light` | Light theme |

## GPU detection — the one genuinely heuristic part

There's no `sysinfo`-equivalent for GPUs on Linux. `src/data/gpu.rs`:

1. Runs `lspci -k`, finds VGA/3D/Display controller lines, and pulls the
   model name + kernel driver from there.
2. For VRAM, branches by driver:
   - `amdgpu` → reads `mem_info_vram_total` from sysfs directly (exact).
   - `nvidia` → shells out to `nvidia-smi`, matched back to the same PCI
     bus ID so a multi-GPU box doesn't get its numbers crossed.
   - `i915`/`xe` (Intel integrated) → reported as
     `<system RAM> (Shared)`, since there is no dedicated pool to query.
   - anything else → "Unknown".

Every step degrades to `Unknown` rather than panicking if a binary isn't
installed or a sysfs file doesn't exist. A missing `lspci` also raises
the app's tool notice with the install-and-refresh recovery path.

## Refresh

The header's refresh button re-runs `SystemSnapshot::collect()` and
confirms with a notification. There's no auto-refresh timer —
everything here is a manual snapshot. If live-updating values are ever
wanted, that's a `cx.spawn` interval around the refresh path.

## Copy

`Copy details` on Overview writes the plain-fact report to the
clipboard and confirms with a success notification.
