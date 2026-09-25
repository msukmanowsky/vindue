// The MCP face of the control API — mounted at /mcp on the same axum server
// that serves the REST API (see src/api.rs, which owns the shared handlers
// these tools delegate to). Tools deliberately skip UI control (no
// open_panel/dismiss): the MCP surface mirrors the automation-relevant
// subset of what Vindue can do, not the interactive one.
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api;
use crate::config;

const INSTRUCTIONS: &str = "Vindue tiles macOS windows onto a configurable grid (default 6×6). \
Start with get_state: it reports Accessibility trust, the frontmost window, every monitor with \
its canonical label, and the full config. tile_window moves a window into a preset region \
(left_half, top_right, …) or explicit inclusive grid cells. Keyboard shortcuts bind grid \
selections to keys (set_shortcut / delete_shortcut); use list_monitors labels — twins get \
positional suffixes like \"(left)\"/\"(right)\" — to target a specific display. These tools \
move and resize real windows and edit real settings; check state before acting.";

#[derive(Clone)]
pub struct VindueMcp {
    app: tauri::AppHandle,
    tool_router: ToolRouter<Self>,
}

impl VindueMcp {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VindueMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS)
    }
}

// ---------- tool parameter schemas (AI clients see these via JsonSchema) ----------

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CellsParams {
    /// First grid row of the region (0-based, inclusive).
    start_row: i64,
    /// Last grid row of the region (inclusive).
    end_row: i64,
    /// First grid column of the region (0-based, inclusive).
    start_col: i64,
    /// Last grid column of the region (inclusive).
    end_col: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
enum TileAppParam {
    /// Process id of the app to tile.
    Pid(i32),
    /// App name (exact match first, else substring; case-insensitive).
    Name(String),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TileParams {
    /// Named region: full, left_half, right_half, top_half, bottom_half,
    /// top_left, top_right, bottom_left, bottom_right.
    preset: Option<String>,
    /// Explicit grid cells — use instead of preset.
    cells: Option<CellsParams>,
    /// Monitor to tile on: canonical label from list_monitors (e.g.
    /// "DELL U2720Q (left)"), a monitor index, or "current". Defaults to the
    /// display holding the target window.
    monitor: Option<String>,
    /// App to tile — pid or name. Defaults to the frontmost window.
    app: Option<TileAppParam>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetShortcutParams {
    /// KeyboardEvent.code of the key, e.g. "Digit1", "KeyT", "Backquote".
    key: String,
    /// Grid cells (inclusive) the shortcut tiles into.
    selection: CellsParams,
    /// Monitor label from list_monitors or index to pin this shortcut to a
    /// display. Omit for a relative shortcut that follows whichever monitor
    /// the panel is on.
    monitor: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct KeyParams {
    /// KeyboardEvent.code of the key, e.g. "Digit1".
    key: String,
}

#[derive(Deserialize, JsonSchema)]
struct GapParams {
    /// Horizontal gap between tiled windows, px 0-300.
    width: i64,
    /// Vertical gap between tiled windows, px 0-300.
    height: i64,
}

#[derive(Deserialize, JsonSchema)]
struct MarginsParams {
    /// Margin inset from the top of the work area, px 0-300.
    top: i64,
    /// Margin inset from the right of the work area, px 0-300.
    right: i64,
    /// Margin inset from the bottom of the work area, px 0-300.
    bottom: i64,
    /// Margin inset from the left of the work area, px 0-300.
    left: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SetGridParams {
    /// Grid rows, 1-12. Omit to leave unchanged.
    rows: Option<i64>,
    /// Grid columns, 1-12. Omit to leave unchanged.
    cols: Option<i64>,
    /// Gap between tiled windows, px 0-300.
    window_gap: Option<GapParams>,
    /// Margins inset from the screen edges, px 0-300.
    screen_margins: Option<MarginsParams>,
}

#[derive(Deserialize, JsonSchema)]
struct SetHotkeyParams {
    /// Accelerator string like "CommandOrControl+Alt+S", or "" to remove the
    /// global open-panel hotkey.
    keys: String,
}

#[derive(Deserialize, JsonSchema)]
struct AssignmentParams {
    /// "pinned" (new shortcuts bind to the monitor they are created on) or
    /// "relative" (shortcuts follow the panel's monitor).
    assignment: String,
}

// ---------- tools ----------

#[tool_router(router = tool_router)]
impl VindueMcp {
    #[tool(
        name = "get_state",
        description = "Everything about Vindue right now: Accessibility trust, the current \
                       target window (frontmost), all monitors with canonical labels, and the \
                       full config (grid, keybindings, shortcuts, api). Call this first."
    )]
    async fn get_state(&self) -> Result<String, String> {
        api::blocking(self.app.clone(), api::state_json)
            .await
            .map(|v| v.to_string())
    }

    #[tool(
        name = "tile_window",
        description = "Move and resize a window to a grid region. Presets: full, left_half, \
                       right_half, top_half, bottom_half, top_left, top_right, bottom_left, \
                       bottom_right — or pass explicit inclusive cells. Targets the frontmost \
                       window by default; pass an app name or pid for another app. The monitor \
                       defaults to the one holding the target window."
    )]
    async fn tile_window(&self, Parameters(p): Parameters<TileParams>) -> Result<String, String> {
        let req = api::TileReq {
            preset: p.preset,
            cells: p.cells.map(|c| config::Selection {
                start_row: c.start_row,
                end_row: c.end_row,
                start_col: c.start_col,
                end_col: c.end_col,
            }),
            monitor: p.monitor,
            app: p.app.map(|a| match a {
                TileAppParam::Pid(pid) => api::TileAppRef::Pid(pid),
                TileAppParam::Name(name) => api::TileAppRef::Name(name),
            }),
        };
        api::blocking(self.app.clone(), move |app| api::tile(app, req))
            .await
            .map(|v| v.to_string())
    }

    #[tool(
        name = "list_shortcuts",
        description = "The shortcuts config: assignment mode (pinned binds new shortcuts to the \
                       monitor they are created on; relative follows the panel's monitor) plus \
                       every key → { monitor, selection } binding."
    )]
    async fn list_shortcuts(&self) -> Result<String, String> {
        api::blocking(self.app.clone(), |app| {
            api::read_full_config(app).map(|c| c["shortcuts"].clone())
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "set_shortcut",
        description = "Create or update a keyboard shortcut that tiles the frontmost window \
                       into the given grid cells. Pass a monitor label from list_monitors (or \
                       index) to pin it to a display; omit monitor for a relative shortcut."
    )]
    async fn set_shortcut(
        &self,
        Parameters(p): Parameters<SetShortcutParams>,
    ) -> Result<String, String> {
        api::blocking(self.app.clone(), move |app| {
            // Resolve to the canonical { name, index } pair the app stores;
            // null monitor = relative (follows the panel's display).
            let mon_ref: Value = match p.monitor.as_deref() {
                None | Some("current") | Some("relative") => Value::Null,
                Some(s) => {
                    let mons = api::monitors_with_labels(app)?;
                    if mons.is_empty() {
                        return Err("no monitors reported by the system".into());
                    }
                    let idx = match s.parse::<usize>() {
                        Ok(i) if i < mons.len() => i,
                        _ => mons
                            .iter()
                            .position(|m| m.label == s)
                            .or_else(|| {
                                let bare = config::strip_label_suffix(s);
                                mons.iter().position(|m| m.label == bare)
                            })
                            .ok_or_else(|| {
                                format!(
                                    "no monitor matches \"{s}\" (available: {})",
                                    mons.iter()
                                        .map(|m| m.label.clone())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                            })?,
                    };
                    json!({ "name": mons[idx].label, "index": idx })
                }
            };
            let def = json!({
                "monitor": mon_ref,
                "selection": {
                    "startRow": p.selection.start_row,
                    "endRow": p.selection.end_row,
                    "startCol": p.selection.start_col,
                    "endCol": p.selection.end_col,
                },
            });
            api::put_shortcut_key(app, &p.key, def)
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "delete_shortcut",
        description = "Remove a shortcut binding by its KeyboardEvent.code. Reports whether it \
                       existed."
    )]
    async fn delete_shortcut(
        &self,
        Parameters(p): Parameters<KeyParams>,
    ) -> Result<String, String> {
        api::blocking(self.app.clone(), move |app| {
            api::delete_shortcut_key(app, &p.key).map(|d| json!({ "deleted": d }))
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "get_config",
        description = "The full config: grid (rows, cols, windowGap, screenMargins), keybindings \
                       (openPanel hotkey; \"\" = none), shortcuts (assignment + keys), api \
                       (port, enabled)."
    )]
    async fn get_config(&self) -> Result<String, String> {
        api::blocking(self.app.clone(), api::read_full_config)
            .await
            .map(|v| v.to_string())
    }

    #[tool(
        name = "set_grid",
        description = "Update the grid: rows/cols (1-12), windowGap and screenMargins (px \
                       0-300). Only the fields passed change. Stored shortcuts that no longer \
                       fit are rescaled proportionally, and open panel/Settings views refresh."
    )]
    async fn set_grid(&self, Parameters(p): Parameters<SetGridParams>) -> Result<String, String> {
        let mut patch = serde_json::Map::new();
        if let Some(r) = p.rows {
            patch.insert("rows".into(), json!(r));
        }
        if let Some(c) = p.cols {
            patch.insert("cols".into(), json!(c));
        }
        if let Some(g) = p.window_gap {
            patch.insert(
                "windowGap".into(),
                json!({ "width": g.width, "height": g.height }),
            );
        }
        if let Some(m) = p.screen_margins {
            patch.insert(
                "screenMargins".into(),
                json!({ "top": m.top, "right": m.right, "bottom": m.bottom, "left": m.left }),
            );
        }
        api::blocking(self.app.clone(), move |app| {
            api::patch_config(app, "grid", Value::Object(patch))
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "set_margins",
        description = "Set screen margins (px 0-300) inset from each edge of every display's \
                       work area."
    )]
    async fn set_margins(
        &self,
        Parameters(m): Parameters<MarginsParams>,
    ) -> Result<String, String> {
        api::blocking(self.app.clone(), move |app| {
            api::patch_config(
                app,
                "grid",
                json!({ "screenMargins": { "top": m.top, "right": m.right, "bottom": m.bottom, "left": m.left } }),
            )
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "set_hotkey",
        description = "Set or clear the global hotkey that opens the Vindue panel (e.g. \
                       \"CommandOrControl+Alt+S\"; \"\" = none). Registration fails if macOS or \
                       another app owns the combo — the error will say so."
    )]
    async fn set_hotkey(
        &self,
        Parameters(p): Parameters<SetHotkeyParams>,
    ) -> Result<String, String> {
        api::blocking(self.app.clone(), move |app| {
            api::patch_config(app, "keybindings", json!({ "openPanel": p.keys }))
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "set_shortcut_assignment",
        description = "Set whether new shortcuts are \"pinned\" to the monitor they are created \
                       on or \"relative\" (following the panel's monitor)."
    )]
    async fn set_shortcut_assignment(
        &self,
        Parameters(p): Parameters<AssignmentParams>,
    ) -> Result<String, String> {
        api::blocking(self.app.clone(), move |app| {
            api::patch_config(app, "shortcuts", json!({ "assignment": p.assignment }))
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "list_monitors",
        description = "All connected displays: canonical label (use these strings to target \
                       monitors — twins get positional suffixes like \"(left)\"/\"(right)\"), \
                       position, size, work area, and scale factor."
    )]
    async fn list_monitors(&self) -> Result<String, String> {
        api::blocking(self.app.clone(), |app| {
            api::monitors_with_labels(app).map(|m| api::mons_json(&m))
        })
        .await
        .map(|v| v.to_string())
    }

    #[tool(
        name = "list_apps",
        description = "Running apps that own windows: pid and name — pass either to \
                       tile_window's app parameter."
    )]
    async fn list_apps(&self) -> Result<String, String> {
        api::blocking(self.app.clone(), api::apps_json)
            .await
            .map(|v| v.to_string())
    }
}

/// Build the streamable-HTTP MCP service for the axum router. rmcp enforces
/// its own Host allowlist (localhost/127.0.0.1/::1) on every request.
pub fn service(app: tauri::AppHandle) -> api::McpService {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };
    StreamableHttpService::new(
        move || Ok(VindueMcp::new(app.clone())),
        std::sync::Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    )
}

/// The full tool catalog exactly as `tools/list` serves it to clients — built
/// from the static router, no app handle required. Consumed by the `docgen`
/// bin; the CI drift gate keeps the committed website copy byte-identical to
/// what this returns, so the human docs can't diverge from the protocol
/// surface AI clients actually see.
pub fn tool_catalog() -> Vec<rmcp::model::Tool> {
    VindueMcp::tool_router().list_all()
}
