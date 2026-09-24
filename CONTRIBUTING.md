# Contributing to Vindue

Thanks for helping make Vindue better! This is a small, focused project and
contributions of any size are welcome.

## What Vindue is

A grid-based window tiler for macOS (inspired by Divvy), built with Tauri 2
(React + TypeScript front end, Rust back end). Its distinguishing feature is
a built-in automation surface: a loopback HTTP API (`/api/v1/*`) and an MCP
server (`/mcp`) so scripts and AI clients can drive window management.

## Development setup

Requirements: macOS, Node.js 20+, Rust (stable), Xcode command line tools.

```sh
git clone https://github.com/msukmanowsky/vindue.git
cd vindue
npm install
npm run tauri dev      # runs the app in dev mode
```

The app needs **Accessibility** permission to move windows (System Settings →
Privacy & Security → Accessibility). Dev builds re-trigger this prompt when
the binary path changes.

## Running tests

```sh
npm run verify                # every gate: tsc+vite, vitest, cargo fmt/clippy/test
```

Pieces, when iterating:

```sh
npm test                          # vitest (TypeScript suites)
(cd src-tauri && cargo test)      # Rust suites
npm run build                     # tsc + vite (type check)
(cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings)
npm run website:build             # docs site (after one-time `cd website && npm install`)
```

CI runs all of the above on every PR. Please keep it green.

## The fixture-parity rule (important)

Pure logic (validation, rescaling, cell→rect math, presets, monitor labels,
monitor resolution) is pinned by **golden-vector fixtures** in `fixtures/*.json`.
Each fixture file is consumed by **both** test suites:

- TypeScript: `src/fixtures.test.ts` (vitest)
- Rust: tests in `src-tauri/src/config.rs` via `include_str!("../../fixtures/…")`

If you change any pure logic: update the fixtures **once** and both suites
must pass against the same vectors. If you add new pure logic: add cases to
the relevant fixture file rather than only one language's tests. Never fix a
failing fixture test by editing only one side.

## Architecture quick map

| Area | Where |
|---|---|
| Panel UI (drag-to-select grid) | `src/App.tsx`, `src/geometry.ts` |
| Settings window (schema-validated forms) | `src/Settings.tsx`, `src/configSchema.ts` |
| Config persistence | `src/store.ts`, `src-tauri/src/config.rs` (Rust seed/validation/pure ports) |
| Shortcuts + global hotkey | `src/shortcuts.ts`, `src/HotkeyInput.tsx`, `src/bindings.ts` |
| Native macOS glue (AX, displays, strips) | `src-tauri/src/ax.rs`, `src-tauri/src/lib.rs` |
| HTTP API | `src-tauri/src/api.rs` |
| MCP server | `src-tauri/src/mcp.rs` |

Config lives at `~/Library/Application Support/com.oddinteractive.vindue/config.json`
(namespaced v1 shape: `grid`, `keybindings`, `shortcuts`, `api`). The Rust seed
and the TS default are pinned to the same shape from both directions — see
`src/store.test.ts` and the `default_config_is_the_documented_shape` Rust test.

## Pull requests

- Small, focused PRs; describe the "why" in the description.
- Add or update fixtures/tests for logic changes.
- Update `README.md` (and website docs, once present) for user-visible changes.
- No comments-only churn; match the existing code style.

## Security

Please do **not** open public issues for security problems — see
[SECURITY.md](SECURITY.md).

## License

By contributing, you agree your contributions will be licensed under the
[MIT License](LICENSE).
