// The loopback HTTP control API — and the mount point for the MCP endpoint
// (src/mcp.rs). One axum server, two faces over shared handlers:
//   - REST  /api/v1/*  for scripts and humans (curl-able, JSON in/out)
//   - MCP   /mcp       for AI clients (rmcp streamable HTTP, official SDK)
//
// Security model (no auth, by design — see README "HTTP API"):
//   - bound to 127.0.0.1 only;
//   - Host-header allowlist (DNS-rebinding defense), enforced here for REST
//     and by rmcp's own validation for /mcp;
//   - browser-originated requests rejected: browsers always attach Origin or
//     Sec-Fetch-Site headers (even for simple requests), while curl/scripts/
//     MCP clients never do — so a web page cannot drive Vindue.
//
// REST and MCP tools both call the pub(crate) helpers below; the helpers do
// the real work against the same internals the Tauri commands use.
use axum::{
    extract::{Path, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use crate::config::{self, ApiCfg};

pub type McpService = rmcp::transport::streamable_http_server::StreamableHttpService<
    crate::mcp::VindueMcp,
    rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
>;

// ---------- server lifecycle ----------

#[derive(Default)]
pub struct ApiServerState {
    inner: Mutex<ApiInner>,
}

#[derive(Default)]
struct ApiInner {
    task: Option<tauri::async_runtime::JoinHandle<()>>,
    bound_port: Option<u16>,
    cfg: Option<ApiCfg>,
}

/// Start/stop/rebind the loopback server to match `cfg`. The swap runs after
/// a short delay on a separate task so an in-flight request that caused the
/// change (PUT /api/v1/config/api) can flush its response before we abort
/// the listener it arrived on.
pub fn apply_server_config(app: &tauri::AppHandle, cfg: Option<ApiCfg>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        {
            let state = app.state::<ApiServerState>();
            let mut inner = state.inner.lock().unwrap();
            if inner.cfg == cfg {
                return;
            }
            if let Some(t) = inner.task.take() {
                t.abort();
            }
            inner.bound_port = None;
            inner.cfg = cfg;
        }
        if let Some(c) = cfg {
            if c.enabled {
                start(&app, c.port).await;
            }
        }
    });
}

async fn start(app: &tauri::AppHandle, preferred: u16) {
    let mut picked: Option<(tokio::net::TcpListener, u16)> = None;
    for port in preferred..=preferred.saturating_add(10) {
        if let Ok(l) = tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            picked = Some((l, port));
            break;
        }
    }
    let Some((listener, bound)) = picked else {
        log::error!("API: could not bind 127.0.0.1:{preferred} (tried 10 successors)");
        return;
    };
    let router = router(app.clone());
    let task = tauri::async_runtime::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            log::error!("API server stopped: {e}");
        }
    });
    let state = app.state::<ApiServerState>();
    let mut inner = state.inner.lock().unwrap();
    inner.task = Some(task);
    inner.bound_port = Some(bound);
    log::info!("API listening on http://127.0.0.1:{bound} (REST /api/v1, MCP /mcp)");
}

/// Live server status for the Settings UI.
#[tauri::command]
pub fn api_info(state: tauri::State<ApiServerState>) -> Value {
    let inner = state.inner.lock().unwrap();
    json!({
        "enabled": inner.cfg.map(|c| c.enabled).unwrap_or(false),
        "configuredPort": inner.cfg.map(|c| c.port),
        "boundPort": inner.bound_port,
        "running": inner.task.is_some(),
    })
}

// ---------- guards ----------

async fn guard(req: Request, next: Next) -> Response {
    let h = req.headers();
    // Browsers always attach these on cross-document requests; scripts and
    // MCP clients never do. Rejecting them blocks drive-by web abuse of the
    // unauthenticated loopback API.
    if h.contains_key(header::ORIGIN) || h.contains_key("sec-fetch-site") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "browser-originated requests are rejected" })),
        )
            .into_response();
    }
    if let Some(host) = h.get(header::HOST).and_then(|v| v.to_str().ok()) {
        let bare = host.rsplit_once(':').map(|(b, _)| b).unwrap_or(host);
        if !matches!(bare, "127.0.0.1" | "localhost") {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Host header not allowed" })),
            )
                .into_response();
        }
    }
    next.run(req).await
}

/// `curl -d` defaults to Content-Type: application/x-www-form-urlencoded,
/// and axum's Json extractor rejects anything but application/json. This is
/// a curl-first local API, so REST bodies are treated as JSON regardless of
/// the header (MCP clients set it correctly themselves; /mcp is not nested
/// under this layer).
async fn json_content(mut req: Request, next: Next) -> Response {
    req.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    next.run(req).await
}

// ---------- shared helpers (REST + MCP both call these) ----------

/// The full config as one JSON value, with defaults filled for any missing
/// top-level key (fresh or pre-v2 stores).
pub fn read_full_config(app: &tauri::AppHandle) -> Result<Value, String> {
    let store = app.store(crate::CONFIG_FILE).map_err(|e| e.to_string())?;
    let mut out = serde_json::Map::new();
    for (k, dv) in config::default_config_json()
        .as_object()
        .expect("defaults are an object")
    {
        out.insert(k.clone(), store.get(k).unwrap_or_else(|| dv.clone()));
    }
    Ok(Value::Object(out))
}

fn deep_merge(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            let mut out = b.clone();
            for (k, v) in p {
                let cur = b.get(k).cloned().unwrap_or(Value::Null);
                out.insert(k.clone(), deep_merge(&cur, v));
            }
            Value::Object(out)
        }
        (_, p) => p.clone(),
    }
}

/// Merge `patch` into config section `section` and run the full commit
/// pipeline (validate → store → broadcast → hotkey swap → API rebind).
pub fn patch_config(app: &tauri::AppHandle, section: &str, patch: Value) -> Result<Value, String> {
    let mut full = read_full_config(app)?;
    let cur = full.get(section).cloned().unwrap_or(Value::Null);
    full[section] = deep_merge(&cur, &patch);
    crate::commit_config_inner(app, full.clone())?;
    Ok(full)
}

pub fn put_shortcut_key(app: &tauri::AppHandle, key: &str, def: Value) -> Result<Value, String> {
    if !config::is_assignable_key(key) {
        return Err(format!("\"{key}\" is not an assignable key"));
    }
    let mut full = read_full_config(app)?;
    // commit_config validates the definition shape and grid bounds.
    full["shortcuts"]["keys"][key] = def;
    crate::commit_config_inner(app, full.clone())?;
    Ok(full)
}

pub fn delete_shortcut_key(app: &tauri::AppHandle, key: &str) -> Result<bool, String> {
    let mut full = read_full_config(app)?;
    let existed = full["shortcuts"]["keys"]
        .as_object_mut()
        .map(|m| m.remove(key).is_some())
        .unwrap_or(false);
    if existed {
        crate::commit_config_inner(app, full)?;
    }
    Ok(existed)
}

/// One connected monitor with its canonical label (twins disambiguated).
pub struct MonInfo {
    pub index: usize,
    pub label: String,
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_w: u32,
    pub work_h: u32,
    pub scale: f64,
}

fn main_names(app: &tauri::AppHandle) -> Vec<(u32, String)> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(crate::ax::monitor_names_main());
            })
            .is_err()
        {
            return Vec::new();
        }
        rx.recv().unwrap_or_default()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Vec::new()
    }
}

fn main_apps(app: &tauri::AppHandle) -> Result<Vec<(i32, String)>, String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(crate::ax::running_apps_main());
            })
            .is_err()
        {
            return Err("main-thread dispatch failed".into());
        }
        Ok(rx.recv().unwrap_or_default())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(Vec::new())
    }
}

pub fn monitors_with_labels(app: &tauri::AppHandle) -> Result<Vec<MonInfo>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let name_map: HashMap<u32, String> = main_names(app).into_iter().collect();
    let label_mons: Vec<config::LabelMon> = monitors
        .iter()
        .map(|m| config::LabelMon {
            name: m.name().map(|s| s.to_string()),
            x: m.position().x,
            y: m.position().y,
        })
        .collect();
    let labels = config::monitor_labels(&label_mons, &name_map);
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let wa = m.work_area();
            MonInfo {
                index: i,
                label: labels
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("Display {}", i + 1)),
                name: m.name().map(|s| s.to_string()),
                x: m.position().x,
                y: m.position().y,
                w: m.size().width,
                h: m.size().height,
                work_x: wa.position.x,
                work_y: wa.position.y,
                work_w: wa.size.width,
                work_h: wa.size.height,
                scale: m.scale_factor(),
            }
        })
        .collect())
}

pub(crate) fn mons_json(mons: &[MonInfo]) -> Value {
    json!(mons
        .iter()
        .map(|m| json!({
            "index": m.index,
            "label": m.label,
            "name": m.name,
            "position": { "x": m.x, "y": m.y },
            "size": { "width": m.w, "height": m.h },
            "workArea": {
                "position": { "x": m.work_x, "y": m.work_y },
                "size": { "width": m.work_w, "height": m.work_h },
            },
            "scaleFactor": m.scale,
        }))
        .collect::<Vec<_>>())
}

pub fn ax_trusted_now() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::ax::is_trusted(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn target_json(_app: &tauri::AppHandle) -> Result<Value, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(match crate::ax::frontmost_target() {
            Some(t) => serde_json::to_value(&t).map_err(|e| e.to_string())?,
            None => Value::Null,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Value::Null)
    }
}

pub fn apps_json(app: &tauri::AppHandle) -> Result<Value, String> {
    let apps = main_apps(app)?;
    Ok(json!(apps
        .iter()
        .map(|(pid, name)| json!({ "pid": pid, "name": name }))
        .collect::<Vec<_>>()))
}

pub fn state_json(app: &tauri::AppHandle) -> Result<Value, String> {
    let mons = monitors_with_labels(app)?;
    Ok(json!({
        "axTrusted": ax_trusted_now(),
        "target": target_json(app)?,
        "monitors": mons_json(&mons),
        "config": read_full_config(app)?,
    }))
}

// ---------- tiling ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileReq {
    /// Named region: full, left_half, right_half, top_half, bottom_half,
    /// top_left, top_right, bottom_left, bottom_right.
    pub preset: Option<String>,
    /// Explicit cells: { startRow, endRow, startCol, endCol }.
    pub cells: Option<config::Selection>,
    /// Monitor: omitted/"current" = the display holding the target window;
    /// otherwise a canonical label ("DELL U2720Q (left)") or an index.
    pub monitor: Option<String>,
    /// Target app: pid number or name (exact, else substring, case-insensitive);
    /// omitted = frontmost window.
    pub app: Option<TileAppRef>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum TileAppRef {
    Pid(i32),
    Name(String),
}

/// The tile operation behind POST /api/v1/tile and the tile_window MCP tool:
/// resolve target → resolve monitor → selection → rect → apply via AX.
pub fn tile(app: &tauri::AppHandle, req: TileReq) -> Result<Value, String> {
    let full = read_full_config(app)?;
    let cfg: config::Config =
        serde_json::from_value(full).map_err(|e| format!("stored config is invalid: {e}"))?;

    // 1. Selection.
    let sel = match (req.preset.as_deref(), req.cells) {
        (Some(p), _) => config::preset_selection(p, cfg.grid.rows, cfg.grid.cols)
            .ok_or_else(|| format!("unknown preset \"{p}\""))?,
        (None, Some(c)) => {
            if c.start_row < 0
                || c.start_col < 0
                || c.start_row > c.end_row
                || c.start_col > c.end_col
                || c.end_row >= cfg.grid.rows
                || c.end_col >= cfg.grid.cols
            {
                return Err(format!(
                    "cells must fit within the {}×{} grid, start <= end",
                    cfg.grid.rows, cfg.grid.cols
                ));
            }
            c
        }
        (None, None) => return Err("provide either \"preset\" or \"cells\"".into()),
    };

    // 2. Target window.
    #[cfg(target_os = "macos")]
    let target = match req.app {
        Some(TileAppRef::Pid(pid)) => {
            crate::ax::target_for_pid(pid).ok_or("no window found for that pid")?
        }
        Some(TileAppRef::Name(name)) => {
            let apps = main_apps(app)?;
            let lower = name.to_lowercase();
            let pid = apps
                .iter()
                .find(|(_, n)| n.to_lowercase() == lower)
                .or_else(|| apps.iter().find(|(_, n)| n.to_lowercase().contains(&lower)))
                .map(|(pid, _)| *pid)
                .ok_or_else(|| format!("no running app matches \"{name}\""))?;
            crate::ax::target_for_pid(pid).ok_or("no window found for that app")?
        }
        None => crate::ax::frontmost_target().ok_or("no target window — focus one first")?,
    };
    #[cfg(not(target_os = "macos"))]
    let target = {
        let _ = req.app;
        return Err("Windows port is a post-eval phase".into());
    };

    // 3. Monitor.
    let mons = monitors_with_labels(app)?;
    if mons.is_empty() {
        return Err("no monitors reported by the system".into());
    }
    let mon_idx = match req.monitor.as_deref() {
        None | Some("current") => {
            let (cx, cy) = (target.x + target.w / 2.0, target.y + target.h / 2.0);
            let tauri_mons = app.available_monitors().map_err(|e| e.to_string())?;
            crate::monitor_index_for_point(&tauri_mons, cx, cy).unwrap_or(0)
        }
        Some(s) => match s.parse::<usize>() {
            Ok(i) if i < mons.len() => i,
            _ => {
                if let Some(i) = mons.iter().position(|m| m.label == s) {
                    i
                } else {
                    let bare = config::strip_label_suffix(s);
                    mons.iter().position(|m| m.label == bare).ok_or_else(|| {
                        format!(
                            "no monitor matches \"{s}\" (available: {})",
                            mons.iter()
                                .map(|m| m.label.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })?
                }
            }
        },
    };
    let m = &mons[mon_idx];

    // 4. Rect + apply.
    #[cfg(target_os = "macos")]
    {
        if !crate::ax::is_trusted(false) {
            return Err(
                "Accessibility permission not granted — click the tray icon, use Grant access… \
                 in the panel, then re-add Vindue.app in System Settings"
                    .into(),
            );
        }
        let area = config::Rect {
            x: m.work_x as f64,
            y: m.work_y as f64,
            width: m.work_w as f64,
            height: m.work_h as f64,
        };
        let rect = config::selection_to_rect(sel, &cfg.grid, area);
        crate::ax::set_window_bounds(
            target.pid,
            rect.x / m.scale,
            rect.y / m.scale,
            rect.width / m.scale,
            rect.height / m.scale,
        )?;
        Ok(json!({
            "applied": { "x": rect.x, "y": rect.y, "width": rect.width, "height": rect.height },
            "monitor": m.label,
            "target": target,
        }))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (target, m, sel, cfg);
        Err("Windows port is a post-eval phase".into())
    }
}

// ---------- REST handlers ----------

/// Run a blocking shared helper off the async worker.
pub(crate) async fn blocking<F, T>(app: tauri::AppHandle, f: F) -> Result<T, String>
where
    F: FnOnce(&tauri::AppHandle) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())?
}

fn respond(r: Result<Value, String>) -> Response {
    match r {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn h_state(State(app): State<tauri::AppHandle>) -> Response {
    respond(blocking(app, state_json).await)
}

async fn h_get_config(State(app): State<tauri::AppHandle>) -> Response {
    respond(blocking(app, read_full_config).await)
}

async fn h_monitors(State(app): State<tauri::AppHandle>) -> Response {
    respond(blocking(app, |a| monitors_with_labels(a).map(|m| mons_json(&m))).await)
}

async fn h_apps(State(app): State<tauri::AppHandle>) -> Response {
    respond(blocking(app, apps_json).await)
}

async fn h_target(State(app): State<tauri::AppHandle>) -> Response {
    respond(blocking(app, target_json).await)
}

async fn h_tile(State(app): State<tauri::AppHandle>, Json(req): Json<TileReq>) -> Response {
    respond(blocking(app, move |a| tile(a, req)).await)
}

async fn h_get_shortcuts(State(app): State<tauri::AppHandle>) -> Response {
    respond(blocking(app, |a| read_full_config(a).map(|c| c["shortcuts"].clone())).await)
}

async fn h_put_grid(State(app): State<tauri::AppHandle>, Json(patch): Json<Value>) -> Response {
    respond(blocking(app, move |a| patch_config(a, "grid", patch)).await)
}

async fn h_put_keybindings(
    State(app): State<tauri::AppHandle>,
    Json(patch): Json<Value>,
) -> Response {
    respond(blocking(app, move |a| patch_config(a, "keybindings", patch)).await)
}

async fn h_put_assignment(
    State(app): State<tauri::AppHandle>,
    Json(patch): Json<Value>,
) -> Response {
    // Accept {"assignment": "pinned"} or a bare string body.
    let patch = match patch {
        Value::String(s) => json!({ "assignment": s }),
        v => v,
    };
    respond(blocking(app, move |a| patch_config(a, "shortcuts", patch)).await)
}

async fn h_put_api(State(app): State<tauri::AppHandle>, Json(patch): Json<Value>) -> Response {
    respond(blocking(app, move |a| patch_config(a, "api", patch)).await)
}

async fn h_put_shortcut(
    State(app): State<tauri::AppHandle>,
    Path(key): Path<String>,
    Json(def): Json<Value>,
) -> Response {
    respond(blocking(app, move |a| put_shortcut_key(a, &key, def)).await)
}

async fn h_delete_shortcut(
    State(app): State<tauri::AppHandle>,
    Path(key): Path<String>,
) -> Response {
    match blocking(app, move |a| delete_shortcut_key(a, &key)).await {
        Ok(existed) => Json(json!({ "deleted": existed })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn h_mcp(Extension(svc): Extension<Arc<McpService>>, req: Request) -> Response {
    svc.handle(req).await.into_response()
}

fn router(app: tauri::AppHandle) -> Router {
    let mcp_svc = Arc::new(crate::mcp::service(app.clone()));
    let rest = Router::new()
        .route("/state", get(h_state))
        .route("/tile", post(h_tile))
        .route("/config", get(h_get_config))
        .route("/config/grid", put(h_put_grid))
        .route("/config/keybindings", put(h_put_keybindings))
        .route("/config/shortcuts/assignment", put(h_put_assignment))
        .route("/config/api", put(h_put_api))
        .route("/shortcuts", get(h_get_shortcuts))
        .route(
            "/shortcuts/{key}",
            put(h_put_shortcut).delete(h_delete_shortcut),
        )
        .route("/monitors", get(h_monitors))
        .route("/apps", get(h_apps))
        .route("/target", get(h_target))
        .layer(middleware::from_fn(json_content));
    Router::new()
        .nest("/api/v1", rest)
        .route("/mcp", any(h_mcp))
        .layer(Extension(mcp_svc))
        .layer(middleware::from_fn(guard))
        .with_state(app)
}

#[cfg(test)]
mod tests {
    // The guard is the entire security model of the unauthenticated loopback
    // API (see the module header): loopback bind + Host allowlist + rejecting
    // browser-originated requests. These tests pin that behavior.
    use super::*;
    use axum::routing::get;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn guarded() -> Router {
        Router::new()
            .route("/x", get(|| async { "ok" }))
            .layer(middleware::from_fn(guard))
    }

    async fn send(builder: axum::http::request::Builder) -> Response {
        let req = builder.uri("/x").body(axum::body::Body::empty()).unwrap();
        guarded().oneshot(req).await.unwrap()
    }

    #[tokio::test]
    async fn plain_loopback_request_passes() {
        let r = send(axum::http::Request::builder().header(header::HOST, "127.0.0.1:47725")).await;
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn localhost_host_passes() {
        let r = send(axum::http::Request::builder().header(header::HOST, "localhost")).await;
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_host_header_passes() {
        // curl and MCP clients always send Host, but its absence is not a
        // browser signal — nothing to allowlist against.
        let r = send(axum::http::Request::builder()).await;
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn foreign_host_rejected() {
        // DNS-rebinding defense: a rebound domain resolves to 127.0.0.1 but
        // the browser still sends the attacker's Host.
        let r = send(axum::http::Request::builder().header(header::HOST, "evil.example.com")).await;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn origin_header_rejected() {
        // Browsers attach Origin on cross-origin and same-origin POSTs;
        // presence alone is the browser signal, whatever the value.
        for origin in ["http://evil.example.com", "http://localhost:1420", "null"] {
            let r = send(axum::http::Request::builder().header(header::ORIGIN, origin)).await;
            assert_eq!(r.status(), StatusCode::FORBIDDEN, "origin={origin}");
        }
    }

    #[tokio::test]
    async fn sec_fetch_site_rejected() {
        // Sec-Fetch-Site rides on every browser request (simple GETs too),
        // where Origin may be absent.
        for site in ["same-origin", "cross-site", "none"] {
            let r = send(axum::http::Request::builder().header("sec-fetch-site", site)).await;
            assert_eq!(r.status(), StatusCode::FORBIDDEN, "site={site}");
        }
    }

    #[tokio::test]
    async fn rejected_requests_explain_themselves() {
        let r = send(axum::http::Request::builder().header(header::ORIGIN, "http://x")).await;
        let body = r.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("error"), "body was: {text}");
    }

    #[tokio::test]
    async fn json_content_accepts_curl_default_content_type() {
        // curl -d sends application/x-www-form-urlencoded; the REST nest
        // must still feed the Json extractor.
        let app = Router::new()
            .route("/echo", post(|Json(v): Json<Value>| async move { Json(v) }))
            .layer(middleware::from_fn(json_content));
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/echo")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(r#"{"preset":"full"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], br#"{"preset":"full"}"#);
    }
}
