# Testing loop

One loop proves backend and frontend together. Every ticket in the
Shelf rebuild extends it instead of inventing its own.

## How it works

- Collection reads routed through `data::fixture::sys_path` re-root
  under `$SYSINFO_SYS_ROOT` when set. Point it at `target/tmp/harness/*`
  trees built by the tests; production never sets it.
- External tools resolve through `PATH`. Tests replace `PATH` with a
  stub `bin/` (or an empty dir for the no-tools fixture), so only
  stubbed helpers exist during collection.
- Env mutation is process-global: hold the serial lock across
  arrange–act–assert (see the `Env` guard in `tests/harness.rs`).
- `sysinfo`/`if-addrs` sources read the live machine. Tests assert
  structural invariants there (sorted, well-formed, in-range), exact
  values only for seam-routed reads and pure parsers/formatters.

## Extending it (per ticket)

1. Add fixture files under a new `Fixture::new(name)` for the states
   the ticket needs (e.g. missing-tool, no-swap).
2. Assert the exact render inputs the ticket's tab consumes.
3. Add goldens via `scripts/capture.sh` once the tab renders.

## Screenshots

`scripts/capture.sh` walks every tab via `SYSINFO_TAB` and stores
baselines under `design/screenshots/`. Needs a display (or
`xvfb-run`); the light-theme dimension arrives with the theme module.
