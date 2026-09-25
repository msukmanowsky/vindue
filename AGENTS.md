# AGENTS.md

Vindue — grid window tiling for macOS. Tauri 2 app: React/TS UI (`src/`) + Rust
native core (`src-tauri/`), docs site (`website/`, Docusaurus). macOS-only for now.

Deep dives: [README.md](README.md) (pitch, repo orientation, architecture table) ·
[CONTRIBUTING.md](CONTRIBUTING.md) (dev setup, fixture-parity rule, PR rules) ·
[SECURITY.md](SECURITY.md) (threat model) · [fixtures/README.md](fixtures/README.md)
(golden-vector contract).

User-facing docs live in `website/docs/` (published at
https://vindue.app). The README deliberately points at the
site instead of duplicating it — user-visible changes update the site first.

## Commands

```sh
npm install                                  # once
npm run verify                               # every app-side gate, one word
npm run verify:all                           # + docs site build
npm run docs:all                             # after touching api.rs/mcp.rs — regenerate API docs
```

`verify` = `npm run build` (tsc + vite) + `npm test` (vitest) +
`npm run check:rust` (cargo fmt --check, clippy `-D warnings`, cargo test).
Run pieces individually while iterating: `npm test`, `npm run build`,
`npm run test:rust`, `npm run check:rust`, `npm run website:build`
(after a one-time `cd website && npm install`).

Single tests: `npm test -- src/geometry.test.ts` or `npm test -- -t 'name'`;
`(cd src-tauri && cargo test focused_window)`.

**Definition of done:** `npm run verify` green (+ `npm run website:build` if
`website/` changed). Do *not* run `npm run tauri build` for verification — it
takes minutes; `(cd src-tauri && cargo check)` validates the Rust side and
parses `tauri.conf.json` in seconds.

## Invariants

- Config version is `1` (`CONFIG_VERSION` in `src-tauri/src/config.rs`, pinned
  across both suites by the fixtures). Keep it 1 until a real schema migration.
- `fixtures/*.json` are shared golden vectors consumed by **both** suites.
  Behavior change → update the fixture first; both sides must pass it.
- All custom native code lives in `src-tauri/src/ax.rs` — the platform seam.
  `config.rs`/`api.rs`/`mcp.rs` stay platform-neutral (a Windows port adds a
  `platform/windows.rs` sibling behind `#[cfg(target_os)]`).
- The API + MCP server bind `127.0.0.1` only, reject any request carrying
  `Origin`/`Sec-Fetch-Site`, and enforce a Host allowlist. Never weaken these —
  SECURITY.md threat model: local processes trusted, browsers the adversary.
  Default port `47725`.
- `src-tauri/tauri.conf.json` is **strict JSON** — comments break the build.
- `src/` + `src-tauri/` are the Tauri ecosystem conventions — don't rename.
- API docs are **generated, not typed**: `website/docs/reference/generated/`
  (OpenAPI spec + MCP tool catalog) comes from the Rust source (`utoipa`
  annotations in `api.rs`, the rmcp router in `mcp.rs`, via the `docgen` bin),
  and `website/docs/reference/http-api/` renders from the spec. Touch the
  API surface → run `npm run docs:all` and commit the regenerated files; CI
  drift gates fail otherwise. Never hand-edit files under those two dirs.
- No new runtime dependencies without justification in the PR; stay on the
  existing stack (Tauri APIs, axum, rmcp, utoipa, Yup).

## Gotchas

- Moving windows needs the macOS Accessibility grant. Dev builds are ad-hoc
  signed: **every rebuild gets a new signing identity**, permanently staling
  the old grant (remove + re-add in System Settings → Privacy). Not a bug;
  signed releases have a stable identity.
- Inspect a running app: `curl -s 127.0.0.1:47725/api/v1/state | jq` (includes
  `axTrusted`). Config: `~/Library/Application Support/com.oddinteractive.vindue/config.json`;
  logs: `~/Library/Logs/com.oddinteractive.vindue/Vindue.log`.
- CI runs Node 22 + stable Rust (`rust-toolchain.toml` pins clippy + rustfmt
  for rustup users, matching CI gates).
- `docusaurus gen-api-docs all` is upstream-broken (fire-and-forget
  `forEach(async …)` — can exit 0 without writing anything). Always use the
  explicit id: `gen-api-docs vindue` / `clean-api-docs vindue`; every repo
  script already does.
- Admonitions need **bracketed titles**: `:::tip[Title]` … `:::`. The legacy
  bare form (`:::tip Title`) silently renders as literal `:::` text under
  `future: {v4: true}` (MDX1-compat parser off). All docs admonitions were
  converted 2026-09-25; keep new ones bracketed.
- `RELEASE_CHECKLIST.md` is a private working doc (gitignored) — never
  reference it from tracked files.
