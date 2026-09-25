<div align="center">

<img src="design/icon-d-tricolor.png" width="128" alt="Vindue app icon">

# Vindue

**Grid window tiling for macOS — the Divvy drag-a-grid interaction, drivable by scripts and AI.**

[![CI](https://github.com/msukmanowsky/vindue/actions/workflows/ci.yml/badge.svg)](https://github.com/msukmanowsky/vindue/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Docs](https://vindue.app/) · [Roadmap](https://vindue.app/docs/reference/roadmap) · [Changelog](CHANGELOG.md)

</div>

An open-source menu-bar tiler built with Tauri 2 + React/TS. ("Divvy" is a
Mizage trademark; this project is unaffiliated.)

## What it does

1. Focus any window, on any display
2. Left-click the menu-bar icon (or an optional global hotkey — none by
   default) → a 6×6 grid panel appears **on every display**, centered and
   sized to that display's aspect ratio, with the target window **outlined in
   blue** and the app's icon + name in the header
3. Drag across cells → the window moves/resizes to that region; the panel
   auto-dismisses (**Esc** cancels)

Panels are true modals: clicking away dismisses them, the outline follows the
target live, and the header retargets another app without dismissing.

- **Shortcuts** — save any drag to a key (color-coded, hover to preview);
  pinned to a display or relative like Divvy → [docs](https://vindue.app/docs/guide/shortcuts)
- **Multi-monitor** — canonical display labels (positional suffixes for
  identical twins); display-bound shortcuts go beyond Divvy → [docs](https://vindue.app/docs/guide/multi-monitor)
- **Settings** — schema-validated form + raw-JSON views of config.json; grid
  resizes rescale saved shortcuts proportionally → [docs](https://vindue.app/docs/guide/settings)
- **Scriptable + AI-drivable** — loopback HTTP API and an MCP server on one
  port (below)

## Install

Universal (Apple Silicon + Intel) dmgs on the **[Releases page](https://github.com/msukmanowsky/vindue/releases)** —
v0.1.0 is in release-candidate stage; until it lands, [build from source](#run-from-source).

Every release carries two independent proofs:

```sh
shasum -a 256 Vindue_x.y.z_universal.dmg                      # must match the SHA256SUMS.txt asset
gh attestation verify Vindue_x.y.z_universal.dmg --owner msukmanowsky   # SLSA provenance: exact commit + this repo's CI identity
```

Release builds are Apple Developer ID-signed + notarized (Gatekeeper-clean)
and MIT-licensed — and you can always build the tagged commit yourself.
Details: [Verifying your download](https://vindue.app/docs/getting-started/intro).

### Accessibility permission

Moving other apps' windows uses the macOS Accessibility API — grant it once
(signed releases keep the grant across updates). The panel shows exactly which
file needs the grant and detects it automatically: [setup guide](https://vindue.app/docs/getting-started/quickstart) ·
[troubleshooting](https://vindue.app/docs/reference/troubleshooting)
(incl. `AXError -25211` and coexisting with macOS's own tiling).

## Run from source

```sh
npm install
npm run tauri dev                # development
npm run tauri build -- --debug   # or build the .app
```

Requirements and the contributor guide: [CONTRIBUTING.md](CONTRIBUTING.md).

## HTTP API

With `api.enabled` (the default), a control API serves on
`http://127.0.0.1:47725` — REST for scripts and humans, MCP for AI clients,
one server. **No auth, by design**: loopback-only bind, `Host` allowlist
(DNS-rebinding defense), and rejection of any request carrying `Origin` or
`Sec-Fetch-Site` — browsers always attach those, curl/scripts/MCP clients
never do, so a web page you visit cannot drive your windows.
([SECURITY.md](SECURITY.md); pinned by tests in `src-tauri/src/api.rs`.)

```sh
curl -s 127.0.0.1:47725/api/v1/state | jq .
curl -s -X POST 127.0.0.1:47725/api/v1/tile -d '{"preset":"left_half"}'
```

14 endpoints — state, tile (9 presets or explicit cells), config CRUD,
shortcut CRUD, monitors, apps, target, and the server's own live OpenAPI spec
→ **[full reference](https://vindue.app/docs/reference/http-api)** ·
[curl tutorial](https://vindue.app/docs/guide/scripting-with-curl)

## MCP

The same server speaks the [Model Context Protocol](https://modelcontextprotocol.io)
at `/mcp` (streamable HTTP, official MCP Rust SDK) — 12 tools for Claude Code,
Claude Desktop, or any MCP client: tile windows, manage shortcuts, tune the
grid, list monitors and apps. One implementation, two protocols: every tool
delegates to the same handler functions as REST, and clients discover
everything live (`tools/list`, plus usage instructions in the `initialize`
response).

```sh
claude mcp add --transport http vindue http://127.0.0.1:47725/mcp
```

→ **[MCP reference](https://vindue.app/docs/reference/mcp)** ·
[tutorial: tile with Claude](https://vindue.app/docs/guide/tile-with-claude)

## Commands

```sh
npm run verify        # every gate in one word: tsc+vite, vitest, cargo fmt + clippy -D warnings + cargo test
npm run verify:all    # + docs site build
```

The pieces:

```sh
npm test          # vitest: geometry, config schema, shortcuts, hotkey rules, golden-vector fixtures (TS side)
npm run build     # tsc typecheck + vite build
cargo test        # (in src-tauri) placement math, config ports pinned to the same fixtures, API guard
```

The TS and Rust implementations of the shared logic (validation, rescale,
rect math, presets, monitor labels/resolution) are pinned to each other by
golden vectors in `fixtures/*.json` — both suites consume every case, so
drift on either side fails the build.

## Known limitations (deliberate)

- macOS only — the Windows port is on the [roadmap](https://vindue.app/docs/reference/roadmap)
- Shortcuts are *local* (panel must be open) — Divvy-style *global* named shortcuts are a later phase
- No live resize-preview rect beyond the target outline (deferred; no `macOSPrivateApi`, all public APIs)
- Fullscreen-Space apps and apps that don't expose AX windows can't be resized (same as Divvy)
- Mixed-DPI multi-monitor: outline/placement are computed per-monitor but not yet torture-tested
- Monitor identity is name+index — rearranging displays in System Settings can soften a binding to index-fallback (footer flags it)
- The control API has no auth token — loopback bind + Host allowlist + browser-request rejection are the whole model (fine for local single-user; anything running as you can already do all of this)
- Config schema is single-revision (`version: 1`) — on mismatch the config reseeds to defaults; `seed_config` marks the seam where migrations will go

## Repository layout

```
src/          panel + settings UI (React/TS)
src-tauri/    native core (Rust) — everything but ax.rs is platform-neutral;
              ax.rs is the macOS seam where a Windows port would slot in
fixtures/     golden vectors shared by both test suites (vitest + cargo)
website/      docs site (Docusaurus)
design/       brand assets — icon masters + menu-bar glyph
```

## Architecture: framework vs custom

Everything except one module is Tauri framework/plugins (expanded version:
[architecture docs](https://vindue.app/docs/reference/architecture)):

| Piece | How |
|---|---|
| Tray icon, menus, click events | Tauri core `TrayIconBuilder` |
| Global hotkey (+ live re-register) | official `tauri-plugin-global-shortcut` |
| Config persistence + auto-migration | official `tauri-plugin-store` + `seed_config` |
| Open config.json in the default editor | official `tauri-plugin-opener`, capability-scoped to `$APPDATA/config.json` |
| Target app icon in the panel header | `NSRunningApplication.icon` drawn into a 64×64 bitmap → PNG (`ax.rs`), raw-byte IPC, cached per pid |
| Panel/settings/strip windows (frameless, always-on-top, hidden toggle, lazy creation) | Tauri window APIs |
| Monitor info for geometry + display-bound shortcuts | `@tauri-apps/api/window` `currentMonitor()` / `availableMonitors()` |
| Friendly display names (*DELL U2720Q*, not `Monitor #41042`) | `NSScreen.localizedName` via objc2 (`ax.rs`), keyed by CGDisplayModelNumber (tao's placeholder number) |
| One panel per display (`panel-N`), each grid matching its display's aspect ratio; focus follows the target's display | dynamic windows + `panel_placement` + `set_size`/`set_position(Logical…)` (Rust) |
| Live outline tracking + click-outside dismissal (true-modal) | watcher thread polling `CGWindowList` (runs only while the panels are visible) |
| App picker (running apps + icons, retarget without dismissing) | `NSWorkspace.runningApplications` + per-pid `CGWindowList` scan (`ax.rs`) |
| Key focus on hotkey activation | `NSApplication.activate` + `makeFirstResponder(contentView)` (`ax.rs`) — tao's show only unhides |
| Grid UI, drag selection, chips, dotted previews, settings form/JSON | React (one bundle, routed by window label) |
| Selection → rect math | pure TS (`src/geometry.ts`) + Rust port (`config.rs`), both pinned by `fixtures/` golden vectors |
| Shortcut keys/colors/monitor resolution | pure TS (`src/shortcuts.ts`) + Rust port, fixture-pinned |
| Config schema enforcement | Yup (`src/configSchema.ts`) + hand-rolled Rust twin (`config.rs`), fixture-pinned |
| Loopback control API (REST `/api/v1`) | `axum` bound to 127.0.0.1 (`src-tauri/src/api.rs`) — Host allowlist + browser-request guard, test-pinned |
| MCP endpoint (`/mcp`) | official MCP Rust SDK (`rmcp`) streamable-HTTP service nested in the same axum router (`src-tauri/src/mcp.rs`) |
| **Custom native glue** | `src-tauri/src/ax.rs`: AX permission check, frontmost-window snapshot + bounds via `CGWindowListCopyWindowInfo`, resize/move via `AXUIElement`, localized display names + app icons via AppKit |

Key ordering detail: the target window is snapshotted in Rust **before** the panel is
shown/focused (`toggle_panel` in `lib.rs`) — otherwise the panel itself would become
the "frontmost window".

---

[Website & docs](https://vindue.app/) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md) · [Sponsor](https://github.com/sponsors/msukmanowsky)
