#!/bin/sh
# Screenshot matrix for the harness: every tab in both themes, captured
# unattended via the SYSINFO_TAB / SYSINFO_THEME launch switches.
#
# Needs a display (or xvfb-run).
# Capture backends tried in order: grim (Wayland), import (X11).
set -eu
cd "$(dirname "$0")/.."

BIN="${BIN:-./target/release/sysinfo-viewer}"
OUT="${OUT:-design/screenshots}"

if [ ! -x "$BIN" ]; then
    echo "build first: cargo build --release" >&2
    exit 1
fi

capture() {
    if command -v grim >/dev/null 2>&1; then
        grim "$1"
    elif command -v import >/dev/null 2>&1; then
        import -window root "$1"
    else
        echo "no capture tool (grim/import); screenshot $1 manually" >&2
        return 1
    fi
}

for theme in dark light; do
    for tab in overview processor memory network storage graphics; do
        echo "== $theme / $tab =="
        mkdir -p "$OUT/$theme"
        SYSINFO_TAB="$tab" SYSINFO_THEME="$theme" "$BIN" &
        pid=$!
        sleep 2
        capture "$OUT/$theme/$tab.png" || true
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    done
done
echo "done: $OUT"
