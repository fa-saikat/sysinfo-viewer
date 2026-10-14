# SysInfo Viewer - System Information Viewer for Linux

[![Build](https://github.com/fa-saikat/sysinfo-viewer/actions/workflows/rust.yml/badge.svg)](https://github.com/fa-saikat/sysinfo-viewer/actions/workflows/rust.yml)
[![Release](https://img.shields.io/github/v/release/fa-saikat/sysinfo-viewer)](https://github.com/fa-saikat/sysinfo-viewer/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A simple system information viewer for the Linux desktop. See your OS, CPU, memory, network, storage, and GPU in one clean window, with search, one-click copy, light/dark themes, and honest fallbacks instead of fake data.

Built with Rust + [GPUI](https://github.com/zed-industries/zed) (via `gpui-kit`).

![Overview - dark theme](design/screenshots/dark/overview.png)

## Why this exists?

I built SysInfo Viewer for [ShopnoOS](https://github.com/fa-saikat/shopno-os), our Debian-based distro and decided to share it. Existing tools either show far more than most people want (hardinfo2), have a dated interface (CPU-X), or work more like a task manager (Mission Center). This one does a single job: show what hardware you have, in a clean layout.

## What it does
- Shows the details of your machine across six tabs.
- Marks anything it cannot detect as `Unknown`.
- Collects on launch and on Refresh, no background polling.
- `Copy details` gives you a plain-text Overview report for bug reports and forum posts.
- The device serial is masked (`••••••••••`) until you explicitly reveal it.

## Features

### Six tabs, zero clutter

| Tab | What you get |
| --- | --- |
| **Overview** | Device model, CPU summary, memory, primary GPU, total storage, serial (masked by default), plus distro, kernel, architecture, hostname, and uptime. `Copy details` copies exactly what's on screen. |
| **Processor** | Model, cores/threads, architecture, virtualization badge (`AMD-V` / `Intel VT-x` / `Unknown`), L3 cache, and frequency facts: min/max frequency, scaling driver, governor, energy preference, and boost. |
| **Memory** | RAM total / used / available with usage bar, plus a separate Swap section that honestly handles the no-swap case. |
| **Network** | Hostname, `Connected` status, and a sorted interface table: type (`Ethernet` / `Wi-Fi` / `Loopback` / `Virtual`), IPv4, MAC, and `Up` / `Down` / `Unknown` state badges. |
| **Storage** | Every mounted device with mount point, device node, filesystem type, used/size figures, per-device usage bars, and a deduplicated total (bind mounts don't double-count). |
| **Graphics** | Detected GPUs with model, VRAM, and kernel driver. Multi-GPU aware: AMD VRAM comes from sysfs, NVIDIA VRAM from `nvidia-smi` matched by PCI bus ID, Intel integrated correctly reports shared memory. |

### Other details

- **Global search:** one toolbar box filters every tab. Press `/` to jump to it, see `N of M` live counts, and the app auto-switches to the first tab with a match.
- **Manual Refresh:** header button re-collects the snapshot and confirms with a notification. The header also shows data freshness (`Just now`, `12s ago`).
- **Light + dark themes:** one-click toggle in the sidebar footer; accent adapts.

## Screenshots

Dark theme (light theme ships the same six views, see `design/screenshots/light/`):

| Overview | Processor |
| --- | --- |
| ![Overview](design/screenshots/dark/overview.png) | ![Processor](design/screenshots/dark/processor.png) |

| Memory | Network |
| --- | --- |
| ![Memory](design/screenshots/dark/memory.png) | ![Network](design/screenshots/dark/network.png) |

| Storage | Graphics |
| --- | --- |
| ![Storage](design/screenshots/dark/storage.png) | ![Graphics](design/screenshots/dark/graphics.png) |

Light-theme Overview for comparison:

![Overview - light theme](design/screenshots/light/overview.png)

## Install

### Requirements

- Linux with X11 or Wayland (Linux-only; no Windows or macOS support planned)
- Rust stable (tested with 1.98) + Cargo
- Optional but recommended for full GPU details:
  - `pciutils` (provides `lspci`)
  - `nvidia-smi` (NVIDIA VRAM only; AMD uses sysfs, Intel iGPU is reported as shared)

### Debian/Ubuntu:

Download the `.deb` from the [latest release](https://github.com/fa-saikat/sysinfo-viewer/releases/latest), then install it:

```sh
sudo apt install ./sysinfo-viewer_*.deb
```

For full GPU details, also install `pciutils` (it provides `lspci`):

```sh
sudo apt install pciutils
```

This installs the binary to `/usr/bin/sysinfo-viewer`, a desktop entry (`System Info Viewer` in your app menu), and the app icon.

### Build and Run from source

```sh
git clone https://github.com/fa-saikat/sysinfo-viewer.git
cd sysinfo-viewer
cargo run --release
```

The binary lands at `./target/release/sysinfo-viewer` with `cargo build --release`. You can also build your own `.deb`:

```sh
git clone https://github.com/fa-saikat/sysinfo-viewer.git
cd sysinfo-viewer
cargo install cargo-deb
./scripts/build-deb.sh
sudo apt install ./target/debian/*.deb
```

## Usage

Launch from your app menu (**System Info Viewer**) or:

```sh
sysinfo-viewer
```

| Action | How |
| --- | --- |
| Switch tabs | Sidebar: Overview, Processor, Memory, Network, Storage, Graphics |
| Search everything | Type in `Search details`, or press `/` anywhere |
| Copy Overview report | `Copy details` (top right on Overview), pastes as `Label: value` lines |
| Refresh data | Refresh icon in the header (re-collects + notifies) |
| Toggle theme | Sun/moon icon in the sidebar footer |
| Reveal serial | `Show` / `Hide` next to Serial number (stays masked by default) |
| About | `About` in the sidebar footer |

Two launch switches help testing and screenshots (dark Overview is the default):

| Variable | Effect |
| --- | --- |
| `SYSINFO_TAB=processor` | Open on that tab (`overview`, `processor`/`cpu`, `memory`/`mem`/`ram`, `network`/`net`, `storage`/`disk`, `graphics`/`gpu`) |
| `SYSINFO_THEME=light` | Open in light theme |

Example:

```sh
SYSINFO_TAB=graphics SYSINFO_THEME=dark sysinfo-viewer
```

## How data is collected

One call, `SystemSnapshot::collect()`, backs every tab. Sources:

- OS / kernel / distro / uptime / hostname via `sysinfo` + OS release info
- CPU model, topology, frequencies, cache, virtualization flags from `/proc/cpuinfo` and `/sys`
- Memory and swap from `sysinfo`
- Network interfaces (name, MAC, state, type) from the system, IPv4 via `if-addrs`
- Storage mounts and usage from the mount table (deduplicated, sorted by mount point)
- GPUs from `lspci -k` (model + driver), then VRAM per driver:
  - `amdgpu` → `mem_info_vram_total` in sysfs (exact)
  - `nvidia` → `nvidia-smi` output matched to the same PCI bus ID
  - `i915` / `xe` → system RAM reported as `(Shared)`
  - anything else → `Unknown`

Every collector degrades to `Unknown` / empty rather than failing the whole snapshot when a file or tool is missing.

## Project layout

```
src/
  main.rs       - app bootstrap (theme, accent, key bindings, window)
  lib.rs        - shared data layer used by the binary and the tests
  theme.rs      - brand accent + light/dark toggle
  assets.rs     - icon registry + app asset source
  tab.rs        - the six tabs: labels, icons, slugs
  search.rs     - search inventory (row texts, counts, auto-switch)
  ui/
    mod.rs      - view module wiring
    root.rs     - app state, header, toolbar, notices, render
    sidebar.rs  - brand mark, navigation, footer (About, theme)
    views.rs    - the six tab bodies
    library.rs  - shared primitives (tiles, tags, badges, bars)
    dialogs.rs  - About dialog
  data/         - collectors + formatters, zero UI dependencies
    mod.rs / fixture.rs - snapshot shape + test fixtures
    os.rs / cpu.rs / memory.rs / network.rs / storage.rs / gpu.rs
tests/
  harness.rs    - integration tests over fixture machines
docs/
  testing.md    - fixture harness guide
design/screenshots/ - dark/ + light/ baselines for all six tabs
scripts/
  build-deb.sh  - build the .deb via cargo-deb + lintian
  capture.sh    - screenshot matrix (all tabs x both themes)
  generate-changelog.sh - changelog helper
packaging/      - .desktop entry, changelog, maintainer scripts
```

## Contributing

PRs welcome. The usual loop:

```sh
cargo test        # unit + full-stack harness (no display needed)
cargo build --release
```

- `cargo test` runs the unit tests and the full-stack harness. It needs no display.
- `docs/testing.md` explains the fixture harness (`SYSINFO_SYS_ROOT` + stub `bin/` on `PATH`).
- `scripts/capture.sh` regenerates `design/screenshots/` (needs a display or `xvfb-run`, plus `grim` on Wayland or `import` on X11). Please review the full tab × theme matrix before calling a UI change done.
- CI (`.github/workflows/rust.yml`) runs `cargo build` + `cargo test` on pushes/PRs to `main`.

Bug reports are most useful when they include the machine's hardware and what the app showed. Detection problems on unusual hardware are especially welcome.

## About the code

This project was written with [OpenCode](https://opencode.ai/) + [Mattpocock's skills](https://www.aihero.dev/skills) and tested by the author. The data layer has fixture-based tests, and CI runs them on every push.

## License & credits

MIT, see [LICENSE](LICENSE).

- Developer: **Fahim A Saikat**
- Copyright: **© 2026 JaduPc**
