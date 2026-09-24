# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-rc.1] - 2026-09-24

Initial release.

### Added

- Grid-based window tiling: a tray click (or an optional global hotkey) opens a
  translucent panel overlay; drag across a configurable grid (default 6×6) to
  resize/move the frontmost window.
- Settings window with schema-validated sections: Grid (rows/cols, window gap,
  screen margins), Keybindings (global "Open Vindue" hotkey), Shortcuts
  (bind grid selections to keys, pinned to a display or relative — following
  the panel's monitor at apply time), and API (enable/port).
- Multi-monitor support with canonical display labels (friendly names plus
  positional suffixes for twins) and a saved-index fallback for display
  re-identification across sessions.
- Nine shortcut presets: full, left/right/top/bottom half, four corners.
- Loopback HTTP control API (`/api/v1/*`): state, tile, config CRUD, shortcut
  CRUD, monitors, apps. Origin/host-guarded; no auth by design (local only).
- MCP server (`/mcp`, streamable HTTP): 12 tools so Claude Code, Claude
  Desktop, and any MCP client can tile windows, manage shortcuts, and read
  configuration.
- Golden-vector fixture suite (`fixtures/*.json`) consumed by both the
  TypeScript (vitest) and Rust (cargo test) test suites.
- Proportional shortcut rescaling when grid dimensions change.
- JSON config at `~/Library/Application Support/com.oddinteractive.vindue/config.json`,
  hand-editable, reseeded on version/shape change.
