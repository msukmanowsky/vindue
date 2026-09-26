<div align="center">

<img src="design/icon-d-tricolor.png" width="128" alt="Vindue app icon">

# Vindue

**Free, open-source grid window tiling for macOS — the Divvy drag-a-grid interaction, programmable by you or AI.**

[![CI](https://github.com/msukmanowsky/vindue/actions/workflows/ci.yml/badge.svg)](https://github.com/msukmanowsky/vindue/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Docs](https://vindue.app/) · [Roadmap](https://vindue.app/docs/roadmap) · [Changelog](CHANGELOG.md)

</div>

<div align="center">

<img src="website/static/img/vindue-demo.gif" width="720" alt="Vindue demo: a hotkey opens a grid panel on every display; dragging across cells tiles the frontmost window there">

<sub>**[▶ Watch the 3-minute tour](https://www.youtube.com/watch?v=jSGT-9tCG1A)**</sub>

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
  pinned to a display or relative → [docs](https://vindue.app/docs/guide/shortcuts)
- **Multi-monitor** — canonical display labels (positional suffixes for
  identical twins); display-bound shortcuts → [docs](https://vindue.app/docs/guide/multi-monitor)
- **Settings** — schema-validated form + raw-JSON views of config.json; grid
  resizes rescale saved shortcuts proportionally → [docs](https://vindue.app/docs/guide/settings)
- **Programmable by you or AI** — loopback HTTP API and an MCP server on one
  port → [docs](https://vindue.app/docs/guide/automating)

## Install

Universal (Apple Silicon + Intel) dmgs on the **[Releases page](https://github.com/msukmanowsky/vindue/releases)**.

Every release carries two independent proofs:

```sh
shasum -a 256 Vindue_x.y.z_universal.dmg                      # must match the SHA256SUMS.txt asset
gh attestation verify Vindue_x.y.z_universal.dmg --owner msukmanowsky   # SLSA provenance: exact commit + this repo's CI identity
```

Release builds are MIT-licensed — and you can always build the tagged commit
yourself. They are **not** Apple Developer ID-signed yet: on first launch
macOS will call the app unidentified — open System Settings → Privacy &
Security → **Open Anyway** (once per release; signing + notarization are
[on the roadmap](https://vindue.app/docs/roadmap)).
Details: [Verifying your download](https://vindue.app/docs/getting-started/quickstart).

### Accessibility permission

Moving other apps' windows uses the macOS Accessibility API — grant it once
per signing identity (while releases stay unsigned, each update gets a fresh
ad-hoc identity, so expect a quick re-grant after updating). The panel shows
exactly which file needs the grant and detects it automatically:
[setup guide](https://vindue.app/docs/getting-started/quickstart) ·
[troubleshooting](https://vindue.app/docs/troubleshooting)
(incl. `AXError -25211` and coexisting with macOS's own tiling).

## Run from source

```sh
npm install
npm run tauri dev                # development
npm run tauri build -- --debug   # or build the .app
```

Requirements and the contributor guide: [CONTRIBUTING.md](CONTRIBUTING.md).

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

## Repository layout

```
src/          panel + settings UI (React/TS)
src-tauri/    native core (Rust) — everything but ax.rs is platform-neutral;
              ax.rs is the macOS seam where a Windows port would slot in
fixtures/     golden vectors shared by both test suites (vitest + cargo)
website/      docs site (Docusaurus)
design/       brand assets — icon masters + menu-bar glyph
```

## Architecture

Everything except one module (`src-tauri/src/ax.rs`, the macOS seam) is Tauri
framework and official plugins — the full framework-vs-custom breakdown lives
in the **[architecture docs](https://vindue.app/docs/architecture)**.

---

[Website & docs](https://vindue.app/) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md) · [Sponsor](https://github.com/sponsors/msukmanowsky)
