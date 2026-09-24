# Vindue

An open-source grid window tiler for macOS — the Divvy drag-a-grid interaction,
drivable by scripts and AI. Built with Tauri 2 + React/TS.
("Divvy" is a Mizage trademark; this project is unaffiliated.)

- Website & docs: https://msukmanowsky.github.io/vindue/ (vindue.app once registered)
- Roadmap: https://msukmanowsky.github.io/vindue/docs/reference/roadmap

Pre-1.0 note: config.json carries a single schema revision (`version: 1`) — on
mismatch the config is reseeded to defaults (no migrations yet; `seed_config`
marks the seam where they'll go).

## What it does

A menu-bar daemon with one interaction loop:

1. Focus any window (on any display)
2. **Left-click the menu bar icon** (or press your global hotkey, if you've recorded
   one in Settings — there is none by default) → a 6×6 grid panel appears
    **on every display — centered and sized to that display's aspect ratio** — with the
    panel on the target's display focused; each header shows the **app's icon + name**,
    and the target window is **outlined in blue** so you can see exactly what will be resized
3. **Drag** across grid cells → the window is moved/resized to that region (work area, margins applied)
4. The panel always auto-dismisses after applying. **Esc** dismisses without applying.

The panels are true modals: clicking any other app's window dismisses them
(~250ms). Move or resize the target and the outline follows. To target a
different app without dismissing, click the app name in the header and pick
from the running-apps list (with icons).

## Shortcuts

Any assignable key can save a grid region — digits `0-9` plus `` ` `` `-` `=` `[` `]` `\` `;` `'` `,` `.` `/`
(physical key codes, so they work regardless of layout; both `` ` `` and `~` count as the same key).

- **Assign**: while the panels are open, press an unassigned key (or **Option+key** to
  reassign) → the footer shows *assigning* with the key's color chip → your next drag is
  **saved and applied in one motion** — the window moves immediately and the panel closes.
  By default assignments are **pinned** to the display
  whose panel you're on — click display B's panel, press the key, drag, and the shortcut
  belongs to B. Settings → Shortcuts can switch assignments to **relative** (no display
  binding; the shortcut follows the panel at apply time, like Divvy).
- **Apply**: press an assigned key while the panel is open → the target window snaps to
  the saved region **on its saved display**, even if that's not where the panel is
  (e.g. `1`/`2` = left/right halves of display A, `3` = left half of display B).
- **Display**: each key gets a unique high-contrast color. Keys belonging to the current
  display show a colored chip on their region's anchor cell — **hover a chip** to see its
  region outlined in a thin dashed line. Keys bound to other displays are listed in the footer
  (*other displays: `3` DELL U2720Q …*) and still fire.
- Shortcuts persist in config.json and are editable in Settings (monitor dropdown +
  mini-grid region editor).

## Multi-monitor

- Panels open on **every display at once**, each centered and sized so its grid mirrors
  that display's aspect ratio (Divvy shows a single panel, on the focused window's
  display). Drag on any panel to place the target on that display; key focus lands on
  the target's display and clicking another panel moves focus there.
- Monitor-bound shortcuts (above) go beyond Divvy, whose shortcuts are always relative
  to the current screen.

### Display identification

macOS gives apps **no stable hardware identity for displays**, so Vindue saves
`{name, index}` per shortcut and resolves best-effort. Why not an ID:

- `CGDirectDisplayID` is assigned per *connection* — it changes on replug/reboot/dock
  cycles, so it can't persist in a config file.
- The closest thing to a hardware ID is the display's **EDID** data via IORegistry
  (what BetterDisplay/`displayplacer` read): manufacturer + product code + serial.
  But the serial is frequently zero or duplicated — two identical monitors typically
  report *identical* EDID — so it's stable-ish yet still not unique-ish, and it costs
  custom IOKit code.
- Displays have no MAC address; that's a network-interface concept.
- The **localized name** (`NSScreen.localizedName` — what System Settings shows, e.g.
  *DELL U2720Q*) is stable across replug/reboot and human-meaningful. It's keyed by
  `CGDisplayModelNumber` — the number tao embeds in its `Monitor #<n>` placeholder names.

Every identifier bottoms out at the same failure mode: **two identical displays are
ambiguous to any API**. Vindue's answer is positional labeling:

- When several connected displays share a name, they're suffixed by their position in
  the arrangement macOS persists: `DELL U2720Q (left)` / `(right)` (or `(middle)` for
  triplets, `(left-to-right #N)` for four+; vertical stacks tiebreak on y). The labels
  appear in the Settings dropdown and are recomputed identically at apply time, so
  "left" means whatever display currently occupies the left slot — matching what you
  see in System Settings. Rearranging displays re-labels them; that's the most durable
  signal macOS exposes.

Resolution chain at apply time: **label match** (incl. positional suffixes) →
**bare-name match** (suffixed ref whose twin is unplugged) → raw tao name (legacy
entries) → **index** → current display, with the panel footer saying so. Settings also
offers *Relative (follows panel)* per shortcut, like Divvy.

## Settings

Tray menu → **Settings…** (or the ⚙ button in the panel). Two views of the same
config.json, switchable at the top-right:

- **Form** (default): sectioned layout — **Grid** (rows/columns 1-12, window
  gap, and screen margins as inputs arranged around a monitor glyph),
  **Keybindings** (*Open Vindue* — a click-to-record hotkey field: click,
  press a modifier+key combination, done. The live hotkey is paused while
  recording, so pressing the current combination records it instead of
  firing; Esc cancels, blur exits. Opinionated admissibility rules reject
  bare keys, Shift-only, Cmd/Alt+text-key, and macOS-reserved combos like
  Cmd+Tab / Cmd+Space, with the reason shown on the spot. ✕ Clear removes
  the hotkey entirely — the panel stays reachable from the menu bar, and
  **no hotkey is the default**),
  **Shortcuts** (how panel assignments save — **Pinned** to the display
  they're created on, the default, or **Relative**, following the panel at
  apply time — plus the shortcut list with color chip, monitor dropdown,
  drag-in-mini-grid region editor, and add-by-pressing-a-key; new shortcuts
  from Add are relative, pinnable per row), and
  **API** (enable toggle + port for the loopback control server, a
  copyable MCP endpoint URL, and live listening status). A successful
  **Save closes the window** — changes apply immediately; failures keep it
  open with inline errors.
- Resizing the grid **rescales saved shortcuts proportionally** (best
  effort): "left half" of 6×6 becomes the left half of 8×8. JSON-view saves
  remap only selections that no longer fit, leaving hand-adjusted ones alone.
- **JSON**: raw config.json for hand-editing. The file's location sits above the
  textarea, with **Copy** and **Open** (launches your default JSON editor).

Both validate against the same Yup schema on save (unknown keys, out-of-range margins,
malformed shortcuts, selections outside the grid → inline errors). Changing the hotkey
re-registers it live, with best-effort conflict detection (Windows reports collisions
reliably; macOS often does not — same caveat Divvy documents).

There is no Reload button: while the window is open, config.json is watched
(file-stamp poll) and external edits — by hand or by assigning shortcuts from
the panel — reload automatically. If you have unsaved edits when the file
changes, nothing is clobbered; you choose to overwrite (Save) or discard.

Left-click the tray icon opens the panel; right-click gives *Open* / *Settings…* / *Quit*. No Dock icon (menu-bar-only app).

## Run it

```sh
npm install
npm run tauri dev                # development
npm run tauri build -- --debug   # or use the built .app:
open src-tauri/target/debug/bundle/macos/Vindue.app
```

### Verifying a download

Releases carry two independent proofs:

1. **Checksum** — `shasum -a 256 Vindue_x.y.z_universal.dmg` must match the
   `SHA256SUMS.txt` asset on the release.
2. **Build provenance (SLSA)** — every dmg is attested via Sigstore in GitHub's
   public transparency log, tying the file's digest to the exact tagged commit
   and this repo's Actions identity:

   ```sh
   gh attestation verify Vindue_x.y.z_universal.dmg --owner msukmanowsky
   ```

The app itself is Apple Developer ID-signed and notarized (Gatekeeper checks on
launch), and Vindue is MIT-licensed — you can always build the tagged commit
yourself.

### Accessibility permission (required)

Moving other apps' windows uses the macOS Accessibility API. On first panel activation the app
shows a banner with the **exact file that needs the grant** (the `.app` or the dev binary —
they differ!) → **Grant access…** opens System Settings **and dismisses the panel** (so the
live target tracker doesn't outline System Settings itself) → remove any old entries with "−",
add the shown file ("+" then ⌘⇧G to paste the path, or drag it from Finder) and enable it →
re-activate the panel; it detects the grant automatically.

**Important:** ad-hoc signed builds get a new code-signing identity on **every rebuild**, so
previous entries go permanently stale — toggling an old entry does nothing. Remove and
re-add. This is macOS TCC behavior for unsigned apps, not a bug — and it is a **dev-build
artifact only**: a properly signed release (Developer ID + notarization) has a stable
identity, so end users grant once and the grant persists across app updates (same as Divvy).

(The target outline and panel placement work *without* the grant — only applying
regions needs it.)

If an apply fails with **`AXError -25211` (kAXErrorAPIDisabled)**, the grant is
missing or went stale mid-session (a rebuild replaced the running binary, or
System Settings was toggled while Vindue was open) — it is *not* the target
app's fault. The panel recovers on its own: the failed apply re-shows the grant
banner and resumes auto-detection, so re-granting fixes it without re-activating.
Check status any time with `curl -s 127.0.0.1:47725/api/v1/state | jq .axTrusted`.
See [Troubleshooting](https://msukmanowsky.github.io/vindue/docs/reference/troubleshooting)
for the full guide and what to include in a bug report.

If you also use macOS's own tiling (hover the green button): its system
overlay (WindowManager) can briefly sit above everything — Vindue looks past
system windows like it to your real frontmost app, so the two coexist.

## Config

`~/Library/Application Support/com.oddinteractive.vindue/config.json` — edit in the
Settings window; all changes apply immediately (the panel live-reloads on save; hotkey
changes re-register live too). Top-level keys mirror the Settings UI sections 1:1.
Defaults are no hotkey, zero gaps/margins, no shortcuts, API on:

```json
{
  "version": 1,
  "grid": {
    "rows": 6,
    "cols": 6,
    "windowGap": { "width": 0, "height": 0 },
    "screenMargins": { "top": 0, "right": 0, "bottom": 0, "left": 0 }
  },
  "keybindings": { "openPanel": "" },
  "shortcuts": {
    "assignment": "pinned",
    "keys": {
      "Digit1": { "monitor": null, "selection": { "startRow": 0, "endRow": 5, "startCol": 0, "endCol": 2 } },
      "Backquote": {
        "monitor": { "name": "DELL U2720Q (left)", "index": 1 },
        "selection": { "startRow": 0, "endRow": 5, "startCol": 0, "endCol": 5 }
      }
    }
  },
  "api": { "port": 47725, "enabled": true }
}
```

- `grid.rows/cols` — 1-12; `grid.windowGap` — gap between adjacent placed windows (each inset by half); `grid.screenMargins` — screen-edge insets (0-300, physical px)
- `keybindings.openPanel` — global panel toggle (parsed by tauri-plugin-global-shortcut, e.g. `Cmd+Shift+Space`, `Ctrl+Alt+D`); `""` = none — the tray icon always opens the panel
- `shortcuts.assignment` — what a panel-mode assignment (key + drag) saves: `"pinned"` (default) binds it to the display it was created on; `"relative"` saves no monitor, so it follows the panel at apply time
- `shortcuts.keys` — key code → `{ monitor, selection }`; `monitor: null` = relative to the panel's display; otherwise `{ name, index }` of the display it was assigned on (`name` = canonical label, see [Display identification](#display-identification)). Easiest created via assignment mode in the panel
- `api` — the loopback HTTP/MCP server: `port` (1024-65535) and `enabled`

## HTTP API

With `api.enabled` (the default), Vindue serves a control API on
`http://127.0.0.1:<api.port>` (default **47725**) — two faces on one server:
REST for scripts and humans, [MCP](#mcp-ai-control) for AI clients.

**Security model — no auth, by design.** The server binds to loopback only,
allowlists the `Host` header (`127.0.0.1`/`localhost` — DNS-rebinding defense),
and **rejects any request carrying `Origin` or `Sec-Fetch-Site` headers**:
browsers always attach them (even on simple GETs), while curl/scripts/MCP
clients never do — so a web page you visit cannot drive your windows. Anything
that can already run code as your user can use this API; it grants no new
capability. (Pinned by tests in `src-tauri/src/api.rs`.)

REST endpoints (JSON in/out; errors are 400 + `{ "error": "…" }`):

| Endpoint | What |
|---|---|
| `GET /api/v1/state` | Everything: AX trust, frontmost target, monitors (canonical labels), full config |
| `POST /api/v1/tile` | The money shot — `{ preset }` or `{ cells }`, optional `monitor` (label/index/"current"), optional `app` (pid or name; default frontmost) |
| `GET /api/v1/config` | Full config |
| `PUT /api/v1/config/grid` | Merge-patch `rows`/`cols`/`windowGap`/`screenMargins` (saved shortcuts rescale proportionally) |
| `PUT /api/v1/config/keybindings` | Merge-patch `{ "openPanel": "Cmd+Alt+S" }` ("" = clear) |
| `PUT /api/v1/config/shortcuts/assignment` | `{ "assignment": "pinned" \| "relative" }` (bare string also accepted) |
| `PUT /api/v1/config/api` | Merge-patch `{ port, enabled }` — rebinds the server itself |
| `GET /api/v1/shortcuts` | Assignment mode + all key bindings |
| `PUT /api/v1/shortcuts/{key}` | Create/update: `{ monitor: label \| null, selection: { startRow, endRow, startCol, endCol } }` |
| `DELETE /api/v1/shortcuts/{key}` | Remove a binding (`{ "deleted": bool }`) |
| `GET /api/v1/monitors` | Displays with canonical labels, positions, work areas, scale factors |
| `GET /api/v1/apps` | Running apps: `{ pid, name }` — usable as `tile`'s `app` |
| `GET /api/v1/target` | Current frontmost window snapshot |

```sh
curl -s 127.0.0.1:47725/api/v1/state | jq .
curl -s -X POST 127.0.0.1:47725/api/v1/tile -d '{"preset":"left_half"}'
curl -s -X POST 127.0.0.1:47725/api/v1/tile -d '{"preset":"top_right","app":"Safari","monitor":"DELL U2720Q (right)"}'
curl -s -X POST 127.0.0.1:47725/api/v1/tile -d '{"cells":{"startRow":0,"endRow":2,"startCol":1,"endCol":4}}'
curl -s -X PUT 127.0.0.1:47725/api/v1/shortcuts/Digit2 -d '{"monitor":null,"selection":{"startRow":0,"endRow":5,"startCol":3,"endCol":5}}'
```

Presets: `full`, `left_half`, `right_half`, `top_half`, `bottom_half`,
`top_left`, `top_right`, `bottom_left`, `bottom_right` — computed
proportionally (`half(n) = max(1, round(n/2))`), so they work on any grid.
`cells` are inclusive 0-based indices and must fit the current grid.
Tiling needs the same Accessibility grant as the panel; the error message
says so when it's missing.

## MCP (AI control)

The same server speaks the [Model Context Protocol](https://modelcontextprotocol.io)
at **`/mcp`** (streamable HTTP, via the official MCP Rust SDK) — so Claude
Code, Claude Desktop, or any MCP client can drive Vindue as a tool.

```sh
claude mcp add --transport http vindue http://127.0.0.1:47725/mcp
```

Tools (settings + tiling; no UI control by design):

| Tool | What |
|---|---|
| `get_state` | AX trust, frontmost window, monitors + labels, full config — call first |
| `tile_window` | preset or explicit cells; optional `monitor` (label/index/"current") and `app` (pid/name) |
| `list_shortcuts` / `set_shortcut` / `delete_shortcut` | manage key → grid-region bindings |
| `get_config` / `set_grid` / `set_margins` | read/tune the grid (shortcuts rescale on resize) |
| `set_hotkey` / `set_shortcut_assignment` | global open-panel hotkey; pinned-vs-relative default |
| `list_monitors` / `list_apps` | canonical display labels; running apps for `tile_window` |

Every tool delegates to the exact same handler functions as the REST API —
one implementation, two protocols. The server advertises usage instructions
in its `initialize` response, so clients learn the workflow (check
`get_state`, use canonical labels, cells are inclusive) without docs.

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

- macOS only — the Windows port is on the [roadmap](https://msukmanowsky.github.io/vindue/docs/reference/roadmap)
- Shortcuts are *local* (panel must be open) — Divvy-style *global* named shortcuts are a later phase
- No live resize-preview rect beyond the target outline (deferred; no `macOSPrivateApi`, all public APIs)
- Fullscreen-Space apps and apps that don't expose AX windows can't be resized (same as Divvy)
- Mixed-DPI multi-monitor: outline/placement are computed per-monitor but not yet torture-tested
- Monitor identity is name+index — rearranging displays in System Settings can soften a binding to index-fallback (footer flags it)
- The control API has no auth token — loopback bind + Host allowlist + browser-request rejection are the whole model (fine for local single-user; anything running as you can already do all of this)

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

Everything except one module is Tauri framework/plugins:

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

[Website & docs](https://msukmanowsky.github.io/vindue/) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md) · [Sponsor](https://github.com/sponsors/msukmanowsky)
