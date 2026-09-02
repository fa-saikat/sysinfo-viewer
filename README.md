# System Information Viewer

A GPUI dashboard for Linux desktop, styled to match the JaduPC 3-in-1
welcome app: same palette (`src/colors.rs`), same full-bleed
background-image-per-section pattern, same rem-based responsive scale.

## Layout

```
src/
  main.rs   — entry point, FsAssets (identical strategy to the welcome app)
  colors.rs — shared palette + the new translucent-card tokens this app needs
  tab.rs    — Tab enum, background_path() per tab, BackgroundAvailability
  app.rs    — all rendering (sidebar, header, six tab views)
  data/
    mod.rs      — SystemSnapshot::collect(), shared formatters
    os.rs       — distro, kernel, hostname, uptime
    cpu.rs      — model, arch, cores/threads, max frequency, virtualization
    memory.rs   — RAM + swap
    network.rs  — interfaces: state, MAC, IP
    storage.rs  — mounted devices + usage
    gpu.rs      — model, VRAM, driver (the trickiest collector — see below)
```

Data collection (`src/data/`) has zero GPUI dependencies. You can pull it
into a `cargo test` or a quick `fn main()` scratch binary on its own to
sanity-check what it reports on your machine before ever touching the UI.

## Before it builds

1. **`gpui`/`gpui_platform` in `Cargo.toml` are placeholders.** The
   welcome app's manifest wasn't part of what got shared, so point these
   at whatever source (git rev, path, registry) your welcome app
   actually pins, so both apps build against one consistent GPUI.
2. **A handful of style methods aren't proven against your pinned GPUI
   version** — `flex_wrap()`, `gpui::relative()`, directional borders
   (`border_r_1()` etc.), `flex_shrink_0()`. Everything else matches
   method-for-method what `main.rs` already demonstrated working. See the
   comment at the top of `app.rs` for the full list.
3. **Drop your background art into `assets/backgrounds/`:**
   `overview.png`, `processor.png`, `memory.png`, `network.png`,
   `storage.png`, `graphics.png`. Any tab missing its file falls back to
   a flat panel color automatically (`tab::BackgroundAvailability`,
   checked once at startup) — no broken-image state, no crash.

## GPU detection — the one genuinely heuristic part

There's no `sysinfo`-equivalent for GPUs on Linux. `src/data/gpu.rs`:

1. Runs `lspci -k`, finds VGA/3D/Display controller lines, and pulls the
   model name + kernel driver from there.
2. For VRAM, branches by driver:
   - `amdgpu` → reads `mem_info_vram_total` from sysfs directly (exact).
   - `nvidia` → shells out to `nvidia-smi`, matched back to the same PCI
     bus ID so a multi-GPU box doesn't get its numbers crossed.
   - `i915`/`xe` (Intel integrated) → reported as "Shared" rather than a
     byte count, since there's no dedicated VRAM to query.
   - anything else → "Unknown".

Every step degrades to `Unknown` rather than panicking if a binary isn't
installed or a sysfs file doesn't exist — but this is the part most
worth running on your actual hardware and adjusting; PCI/sysfs layouts
vary more machine-to-machine than anything else in this app.

## Refresh

The header's refresh button re-runs `SystemSnapshot::collect()` on
click. There's no auto-refresh timer yet — everything here is a manual
snapshot, same as opening the app fresh. If you want live-updating
values (e.g. memory usage ticking every second), that's a `cx.spawn`
interval added around `SysInfoApp::refresh()`; happy to wire that up
next if you want it.

## Not wired up yet

- Scrolling: no tab's content is likely to overflow a normal window, but
  if yours does (lots of storage devices, lots of network interfaces),
  the content area in `app.rs`'s `Render::render` is marked with a
  comment where a scroll container would go.
- Real icons: sidebar/section icons are placeholder glyphs (`◧`, `▤`,
  etc.) rather than an icon font or SVG set, the same way the welcome app
  left room for real art without blocking on having it. Swap
  `Tab::icon_glyph()` for `img("icons/....svg")` once you have icon
  assets, same convention as the background images.
