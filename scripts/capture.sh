#!/bin/sh
# Screenshot matrix for the harness (ticket #8): every tab, captured
# unattended via the SYSINFO_TAB launch switch.
#
# Needs a display (or xvfb-run). The theme dimension arrives with the
# theme module; until then everything captures in the default theme.
# Capture backends tried in order: grim (Wayland), import (X11).
set -eu
cd "$(dirname "$0")/.."

BIN="${BIN:-./target/release/sysinfo-viewer}"
OUT="${OUT:-design/screenshots}"

if [ ! -x "$BIN" ]; then
    echo "build first: cargo build --release" >&2
    exit 1
fi
mkdir -p "$OUT"

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

for tab in overview processor memory network storage graphics; do
    echo "== $tab =="
    SYSINFO_TAB="$tab" "$BIN" &
    pid=$!
    sleep 2
    capture "$OUT/$tab.png" || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
done
echo "done: $OUT"
