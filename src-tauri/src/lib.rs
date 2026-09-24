use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;

mod api;
mod config;
mod mcp;

#[cfg(target_os = "macos")]
mod ax;

const CONFIG_FILE: &str = "config.json";
const STRIP_LABELS: [&str; 4] = ["hl-top", "hl-bottom", "hl-left", "hl-right"];
// Panels are created dynamically, one per display: panel-0, panel-1, …
const PANEL_PREFIX: &str = "panel-";
// Panel sizing budget (points): the grid height the aspect-ratio sizing
// starts from, the chrome around the grid (header + footer + padding + gaps),
// and the horizontal padding added to the grid width.
const GRID_H: f64 = 440.0;
const CHROME_H: f64 = 112.0;
const PAD_X: f64 = 28.0;

#[derive(Clone, Serialize, Default)]
pub struct Target {
    pid: i32,
    app_name: String,
    /// CGWindowNumber — distinguishes windows within the same app.
    wid: i64,
    /// Window bounds in points (global screen space), from CGWindowList.
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Default)]
struct AppState {
    target: Mutex<Option<Target>>,
    /// Set after a successful apply, cleared on panel activation/retarget:
    /// suppresses the target outline so the watcher can't re-light the
    /// strips around the just-moved window while the panels stay open
    /// (the post-save celebration beat, or the instant before dismissal).
    hl_suppressed: AtomicBool,
}

// ---------- commands ----------

#[tauri::command]
fn ax_trusted(prompt: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        ax::is_trusted(prompt)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = prompt;
        true
    }
}

#[tauri::command]
fn current_target(state: tauri::State<AppState>) -> Option<Target> {
    state.target.lock().unwrap().clone()
}

/// Path of the running binary — shown in the Accessibility banner so the user
/// knows exactly which file needs the grant (dev binary vs .app differ!).
#[tauri::command]
fn current_exe() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Apply a rect (physical pixels, in the given monitor's coordinate space)
/// to the snapshotted target window. `scale` converts pixels -> points.
#[tauri::command]
fn apply_rect(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let target = state
        .target
        .lock()
        .unwrap()
        .clone()
        .ok_or("No target window captured — focus a window, then toggle the panel again")?;
    #[cfg(target_os = "macos")]
    {
        let result = ax::set_window_bounds(
            target.pid,
            x / scale,
            y / scale,
            width / scale,
            height / scale,
        );
        if result.is_ok() {
            hide_highlight(&app);
            state.hl_suppressed.store(true, Ordering::SeqCst);
        }
        result
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, target, x, y, width, height, scale);
        Err("Windows port is a post-eval phase".into())
    }
}

/// Hide the panels *and* the highlight strips (frontend dismissals route here).
fn dismiss_panel(app: &tauri::AppHandle) {
    app.state::<AppState>()
        .hl_suppressed
        .store(false, Ordering::SeqCst);
    hide_highlight(app);
    for panel in panels(app) {
        let _ = panel.hide();
    }
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    dismiss_panel(&app);
    // Deactivate so focus returns to the previous app — an Accessory app with
    // no visible windows would otherwise keep swallowing key events.
    #[cfg(target_os = "macos")]
    let _ = app.hide();
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    open_settings_window(&app);
}

/// Fingerprint (mtime + size) of config.json on disk, polled by the Settings
/// window to notice external edits.
#[tauri::command]
fn config_stamp(app: tauri::AppHandle) -> Option<String> {
    let dir = app.path().app_config_dir().ok()?;
    let meta = std::fs::metadata(dir.join(CONFIG_FILE)).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some(format!(
        "{}:{}-{}",
        mtime.as_secs(),
        mtime.subsec_nanos(),
        meta.len()
    ))
}

/// Re-read config.json from disk into the store plugin's in-memory cache.
/// The plugin caches on first load, so external edits are invisible to
/// get()/entries() until this runs.
#[tauri::command]
fn reload_config(app: tauri::AppHandle) -> Result<(), String> {
    let store = app.store(CONFIG_FILE).map_err(|e| e.to_string())?;
    store.reload().map_err(|e| e.to_string())
}

/// Human-friendly display names (NSScreen.localizedName), keyed by
/// CGDisplayModelNumber — the number tao embeds in its "Monitor #<n>"
/// placeholders. AppKit calls must happen on the main thread, hence the hop.
#[tauri::command]
async fn monitor_names(app: tauri::AppHandle) -> Vec<(u32, String)> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(ax::monitor_names_main());
            })
            .is_err()
        {
            log::error!("monitor_names: main-thread dispatch failed");
            return Vec::new();
        }
        let names = rx.recv().unwrap_or_default();
        log::info!("monitor_names: {names:?}");
        names
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Vec::new()
    }
}

/// 64×64 PNG icon of the app with this pid, as raw bytes (empty when the pid
/// is gone or has no icon). Raw-payload response keeps the bytes off the
/// JSON serializer.
#[tauri::command]
async fn app_icon(app: tauri::AppHandle, pid: i32) -> tauri::ipc::Response {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(ax::app_icon_main(pid));
            })
            .is_err()
        {
            log::error!("app_icon: main-thread dispatch failed for pid {pid}");
            return tauri::ipc::Response::new(Vec::new());
        }
        let bytes = rx.recv().unwrap_or(None).unwrap_or_default();
        tauri::ipc::Response::new(bytes)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, pid);
        tauri::ipc::Response::new(Vec::new())
    }
}

/// Running regular apps for the header app-picker: (pid, localized name),
/// sorted by name. NSWorkspace calls must happen on the main thread.
#[tauri::command]
async fn running_apps(app: tauri::AppHandle) -> Vec<(i32, String)> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(ax::running_apps_main());
            })
            .is_err()
        {
            log::error!("running_apps: main-thread dispatch failed");
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

/// Retarget the panels at a specific app's frontmost window (header
/// app-picker). CGWindowList is thread-safe, so no main-thread hop.
#[tauri::command]
fn target_app(app: tauri::AppHandle, pid: i32) -> Option<Target> {
    #[cfg(target_os = "macos")]
    {
        let t = ax::target_for_pid(pid)?;
        *app.state::<AppState>().target.lock().unwrap() = Some(t.clone());
        app.state::<AppState>()
            .hl_suppressed
            .store(false, Ordering::SeqCst);
        show_highlight(&app);
        // Broadcast: every panel updates its header/outline.
        let _ = app.emit("target-changed", t.clone());
        Some(t)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, pid);
        None
    }
}

/// Swap the global panel hotkey, restoring `previous` if the new one can't
/// register. Best-effort conflict detection: Windows reports collisions
/// reliably; macOS may accept a colliding hotkey.
fn swap_hotkey(app: &tauri::AppHandle, hotkey: &str, previous: Option<&str>) -> Result<(), String> {
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    if hotkey.is_empty() {
        // Cleared: no global hotkey; the tray still opens the panel.
        return Ok(());
    }
    let parsed: Shortcut = hotkey
        .parse()
        .map_err(|e| format!("Invalid hotkey \"{hotkey}\": {e:?}"))?;
    if let Err(e) = gs.register(parsed) {
        if let Some(prev) = previous {
            if !prev.is_empty() {
                if let Ok(p) = prev.parse::<Shortcut>() {
                    if let Err(e2) = gs.register(p) {
                        log::error!("failed to restore previous hotkey \"{prev}\": {e2}");
                    }
                }
            }
        }
        return Err(format!(
            "Could not register \"{hotkey}\" — it may be in use by another app or the system. ({e})"
        ));
    }
    Ok(())
}

/// The full config-save pipeline, shared by the Settings UI and the HTTP
/// API: validate (Rust twin of the Yup schema) → store write → broadcast
/// `config-saved` (open panels live-reload) → hotkey swap if changed →
/// API-server rebind if changed.
#[tauri::command]
fn commit_config(app: tauri::AppHandle, config_value: serde_json::Value) -> Result<(), String> {
    commit_config_inner(&app, config_value)
}

fn commit_config_inner(
    app: &tauri::AppHandle,
    mut config_value: serde_json::Value,
) -> Result<(), String> {
    let store = app.store(CONFIG_FILE).map_err(|e| e.to_string())?;
    // When the grid changes size, proportionally remap stored shortcut
    // selections so they still fit — the same behavior the Settings UI
    // applies before saving. Keeps API/MCP grid edits from invalidating
    // existing shortcuts.
    if let Some(g) = store.get("grid") {
        let old_rc = (
            g.get("rows").and_then(|v| v.as_i64()),
            g.get("cols").and_then(|v| v.as_i64()),
        );
        let new_rc = (
            config_value["grid"]["rows"].as_i64(),
            config_value["grid"]["cols"].as_i64(),
        );
        if let ((Some(orows), Some(ocols)), (Some(nrows), Some(ncols))) = (old_rc, new_rc) {
            if (orows, ocols) != (nrows, ncols) {
                if let Some(keys) = config_value["shortcuts"]["keys"].as_object_mut() {
                    for def in keys.values_mut() {
                        if let Ok(sel) =
                            serde_json::from_value::<config::Selection>(def["selection"].clone())
                        {
                            let rescaled =
                                config::rescale_selection(sel, (orows, ocols), (nrows, ncols));
                            if let Ok(v) = serde_json::to_value(rescaled) {
                                def["selection"] = v;
                            }
                        }
                    }
                }
            }
        }
    }
    config::validate_config(&config_value).map_err(|errs| errs.join("; "))?;
    // Snapshot the pieces we need to diff before overwriting.
    let old_hotkey = store.get("keybindings").and_then(|v| {
        v.get("openPanel")
            .and_then(|x| x.as_str())
            .map(String::from)
    });
    let old_api = store
        .get("api")
        .and_then(|v| serde_json::from_value::<config::ApiCfg>(v).ok());
    for (k, v) in config_value
        .as_object()
        .expect("validated config is an object")
    {
        store.set(k.clone(), v.clone());
    }
    store.save().map_err(|e| e.to_string())?;
    // Let any open panel pick the new config up live.
    let _ = app.emit("config-saved", ());

    let new_hotkey = config_value["keybindings"]["openPanel"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if old_hotkey.as_deref() != Some(new_hotkey.as_str()) {
        swap_hotkey(app, &new_hotkey, old_hotkey.as_deref())?;
    }

    let new_api = serde_json::from_value::<config::ApiCfg>(config_value["api"].clone()).ok();
    if new_api != old_api {
        api::apply_server_config(app, new_api);
    }
    Ok(())
}

/// Temporarily enable/disable the global hotkey. Settings disables it while
/// recording a new combination: global shortcuts are intercepted by the OS
/// before any app sees them, so without this the currently registered hotkey
/// would fire instead of flowing to the recorder.
#[tauri::command]
fn set_hotkey_enabled(app: tauri::AppHandle, hotkey: String, enabled: bool) -> Result<(), String> {
    let gs = app.global_shortcut();
    if !enabled || hotkey.is_empty() {
        return gs.unregister_all().map_err(|e| e.to_string());
    }
    let parsed: Shortcut = hotkey
        .parse()
        .map_err(|e| format!("Invalid hotkey \"{hotkey}\": {e:?}"))?;
    gs.register(parsed)
        .map_err(|e| format!("Could not register \"{hotkey}\" ({e})"))
}

// ---------- target-window highlight ----------

/// All live panel windows (one per display, labels `panel-0`, `panel-1`, …).
fn panels(app: &tauri::AppHandle) -> Vec<tauri::WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(PANEL_PREFIX))
        .map(|(_, w)| w)
        .collect()
}

/// Get-or-create a panel window. Mirrors the old static "panel" window config:
/// frameless, always-on-top, hidden until activated; acceptFirstMouse so a
/// click on an unfocused panel interacts immediately (and moves key focus to
/// that display's grid).
fn ensure_panel(app: &tauri::AppHandle, label: &str) -> tauri::Result<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window(label) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Vindue")
        .inner_size(340.0, 408.0)
        .visible(false)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .center()
        .shadow(true)
        .accept_first_mouse(true)
        .build()
}

fn ensure_strip(app: &tauri::AppHandle, label: &str) -> tauri::Result<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window(label) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .shadow(false)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .build()
}

fn hide_highlight(app: &tauri::AppHandle) {
    for label in STRIP_LABELS {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.hide();
        }
    }
}

/// Outline the snapshotted target window with four thin opaque strip windows
/// (no transparency => no private APIs). Strips are created lazily. Uses the
/// CGWindowList bounds from the snapshot, so it works without AX permission.
fn show_highlight(app: &tauri::AppHandle) {
    // Post-apply suppression (see AppState::hl_suppressed): the window has
    // moved; re-outlining it while the panels are still open is jank.
    if app.state::<AppState>().hl_suppressed.load(Ordering::SeqCst) {
        hide_highlight(app);
        return;
    }
    #[cfg(target_os = "macos")]
    {
        // Never outline while the panels are hidden: after an apply, the
        // watcher can fire one last time (250ms poll) before noticing the
        // dismissal, which would leave a phantom outline around the
        // just-moved window.
        if !panels(app).iter().any(|p| p.is_visible().unwrap_or(false)) {
            hide_highlight(app);
            return;
        }
        let target = app.state::<AppState>().target.lock().unwrap().clone();
        let Some(target) = target else {
            hide_highlight(app);
            return;
        };
        let (x, y, w, h) = (target.x, target.y, target.w, target.h);
        if w <= 1.0 || h <= 1.0 {
            hide_highlight(app);
            return;
        }
        let scale = monitor_scale_for_point(app, x + w / 2.0, y + h / 2.0);
        let t = (4.0 * scale).round();
        let (px, py, pw, ph) = (x * scale, y * scale, w * scale, h * scale);
        let rects: [(&str, f64, f64, f64, f64); 4] = [
            ("hl-top", px - t, py - t, pw + 2.0 * t, t),
            ("hl-bottom", px - t, py + ph, pw + 2.0 * t, t),
            ("hl-left", px - t, py, t, ph),
            ("hl-right", px + pw, py, t, ph),
        ];
        for (label, rx, ry, rw, rh) in rects {
            let Ok(win) = ensure_strip(app, label) else {
                continue;
            };
            let _ = win.set_position(PhysicalPosition::new(rx.round() as i32, ry.round() as i32));
            let _ = win.set_size(PhysicalSize::new(rw.round() as u32, rh.round() as u32));
            let _ = win.show();
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// Index of the monitor whose point-space bounds contain the given point
/// (AX/CG coordinates are points; Tauri window placement is physical).
fn monitor_index_for_point(monitors: &[tauri::Monitor], x_pts: f64, y_pts: f64) -> Option<usize> {
    monitors.iter().position(|m| {
        let s = m.scale_factor();
        let mx = m.position().x as f64 / s;
        let my = m.position().y as f64 / s;
        let mw = m.size().width as f64 / s;
        let mh = m.size().height as f64 / s;
        x_pts >= mx && x_pts <= mx + mw && y_pts >= my && y_pts <= my + mh
    })
}

/// The monitor whose point-space bounds contain the given point.
fn monitor_for_point(app: &tauri::AppHandle, x_pts: f64, y_pts: f64) -> Option<tauri::Monitor> {
    let mut monitors = app.available_monitors().ok()?;
    let i = monitor_index_for_point(&monitors, x_pts, y_pts)?;
    Some(monitors.swap_remove(i))
}

/// Scale factor of the monitor containing the point, falling back to the
/// first monitor (e.g. window center briefly off-screen during animations).
fn monitor_scale_for_point(app: &tauri::AppHandle, x_pts: f64, y_pts: f64) -> f64 {
    if let Some(m) = monitor_for_point(app, x_pts, y_pts) {
        return m.scale_factor();
    }
    if let Ok(monitors) = app.available_monitors() {
        if let Some(m) = monitors.first() {
            return m.scale_factor();
        }
    }
    1.0
}

// ---------- activation & windows ----------

/// Toggle the panels. Snapshots the frontmost window *before* any panel is
/// shown, because showing/focusing panels would otherwise steal "frontmost".
/// One panel appears per display, each sized to its display's aspect ratio:
/// drag on any grid to place the target there, and shortcut assignment binds
/// to the panel's display. Key focus goes to the panel on the target's
/// display; clicking another panel moves focus (acceptFirstMouse).
fn toggle_panel(app: &tauri::AppHandle) {
    if panels(app).iter().any(|p| p.is_visible().unwrap_or(false)) {
        dismiss_panel(app);
        // Return focus to the app the user was in.
        #[cfg(target_os = "macos")]
        let _ = app.hide();
        return;
    }

    #[cfg(target_os = "macos")]
    let target = ax::frontmost_target();
    #[cfg(not(target_os = "macos"))]
    let target: Option<Target> = None;
    if let Some(t) = &target {
        *app.state::<AppState>().target.lock().unwrap() = Some(t.clone());
    }

    let Ok(monitors) = app.available_monitors() else {
        return;
    };
    // Destroy stale panels left from activations when more displays existed.
    for (label, w) in app.webview_windows() {
        if let Some(suffix) = label.strip_prefix(PANEL_PREFIX) {
            if suffix.parse::<usize>().is_ok_and(|i| i >= monitors.len()) {
                let _ = w.close();
            }
        }
    }

    for (i, m) in monitors.iter().enumerate() {
        let Ok(panel) = ensure_panel(app, &format!("{PANEL_PREFIX}{i}")) else {
            continue;
        };
        #[cfg(target_os = "macos")]
        let placed = place_panel_on_monitor(&panel, m);
        #[cfg(not(target_os = "macos"))]
        let placed = false;
        if !placed {
            let _ = panel.center();
        }
        let _ = panel.show();
    }

    // Focus the panel on the target's display (fallback: the first).
    let focus_idx = target
        .as_ref()
        .and_then(|t| monitor_index_for_point(&monitors, t.x + t.w / 2.0, t.y + t.h / 2.0))
        .unwrap_or(0);
    if let Some(panel) = app.get_webview_window(&format!("{PANEL_PREFIX}{focus_idx}")) {
        // Activate the app so the panel actually receives key events —
        // hotkey activation alone leaves us visible-but-not-key (tao's
        // app.show() only unhides). Then make the webview first responder:
        // window-key alone doesn't hand keys to WKWebView.
        #[cfg(target_os = "macos")]
        {
            ax::activate_app();
            let _ = panel.set_focus();
            if let Ok(ns) = panel.ns_window() {
                ax::focus_webview(ns);
            }
            // Activation is asynchronous: re-assert focus once it settles,
            // or WKWebView can lose first-responder a beat later.
            let app2 = app.clone();
            let label = format!("{PANEL_PREFIX}{focus_idx}");
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                let app3 = app2.clone();
                let _ = app2.run_on_main_thread(move || {
                    if let Some(p) = app3.get_webview_window(&label) {
                        if p.is_visible().unwrap_or(false) {
                            let _ = p.set_focus();
                            if let Ok(ns) = p.ns_window() {
                                ax::focus_webview(ns);
                            }
                        }
                    }
                });
            });
        }
        #[cfg(not(target_os = "macos"))]
        let _ = panel.set_focus();
    }

    app.state::<AppState>()
        .hl_suppressed
        .store(false, Ordering::SeqCst);
    show_highlight(app);
    start_target_watcher(app);

    let payload = app.state::<AppState>().target.lock().unwrap().clone();
    let _ = app.emit("panel-activated", payload);
}

/// While any panel is visible: (1) true-modal semantics — if the frontmost
/// non-Vindue window changes (the user clicked another app), dismiss;
/// (2) follow the target window's moves/resizes so the outline tracks it.
/// Deliberate retargeting happens via the header app-picker, not by
/// clicking around. CGWindowList needs no AX permission. The thread exits as
/// soon as the panels hide; the flag guarantees a single watcher.
#[cfg(target_os = "macos")]
fn start_target_watcher(app: &tauri::AppHandle) {
    static WATCHING: AtomicBool = AtomicBool::new(false);
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut prev_front: Option<(i32, i64)> = None;
        loop {
            std::thread::sleep(Duration::from_millis(250));
            if !panels(&app).iter().any(|p| p.is_visible().unwrap_or(false)) {
                break;
            }
            // Click-off dismissal: frontmost (non-self) window changed.
            if let Some(f) = ax::frontmost_target() {
                let cur = (f.pid, f.wid);
                if let Some(prev) = prev_front {
                    if prev != cur {
                        dismiss_panel(&app);
                        let _ = app.hide();
                        break;
                    }
                }
                prev_front = Some(cur);
            }
            refresh_target_bounds(&app);
        }
        WATCHING.store(false, Ordering::SeqCst);
    });
}

#[cfg(not(target_os = "macos"))]
fn start_target_watcher(_app: &tauri::AppHandle) {}

/// Poll the target window's bounds (by CGWindowNumber) and refresh the
/// outline + panel headers when it moved or resized.
#[cfg(target_os = "macos")]
fn refresh_target_bounds(app: &tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let mut state = app_state.target.lock().unwrap();
    let Some(t) = state.as_mut() else {
        return;
    };
    let Some((x, y, w, h)) = ax::window_bounds(t.wid) else {
        return;
    };
    if (t.x, t.y, t.w, t.h) == (x, y, w, h) {
        return;
    }
    t.x = x;
    t.y = y;
    t.w = w;
    t.h = h;
    let fresh = t.clone();
    drop(state);
    show_highlight(app);
    // Broadcast: every panel (one per display) updates its header/outline.
    let _ = app.emit("target-changed", fresh);
}

/// Pure panel placement math (unit-tested): given a display's work area in
/// logical points (width, height, origin), return the panel (size, position)
/// in logical points — grid mirroring the display's aspect ratio, centered.
/// None when the work area is too small to be worth placing on.
fn panel_placement(ww: f64, wh: f64, wx: f64, wy: f64) -> Option<(f64, f64, f64, f64)> {
    if ww < 100.0 || wh < 100.0 {
        return None;
    }
    // Window height = grid height + chrome; grid width = grid height * aspect.
    let aspect = (ww / wh).clamp(0.35, 3.6);
    let ph = (wh - 40.0).clamp(320.0, GRID_H + CHROME_H);
    let pw = ((ph - CHROME_H) * aspect + PAD_X).min(ww - 40.0).max(340.0);
    Some((pw, ph, wx + (ww - pw) / 2.0, wy + (wh - ph) / 2.0))
}

/// Size a panel so its grid mirrors this display's aspect ratio and center it
/// on the display's work area. Returns false when there is no sensible
/// placement, so the caller falls back to panel.center().
#[cfg(target_os = "macos")]
fn place_panel_on_monitor(panel: &tauri::WebviewWindow, m: &tauri::Monitor) -> bool {
    let s = m.scale_factor();
    let wa = m.work_area();
    let Some((pw, ph, px, py)) = panel_placement(
        wa.size.width as f64 / s,
        wa.size.height as f64 / s,
        wa.position.x as f64 / s,
        wa.position.y as f64 / s,
    ) else {
        return false;
    };
    let _ = panel.set_size(LogicalSize::new(pw, ph));
    // Logical units: macOS logical == CG global points, and the
    // logical->physical->frame round-trip preserves them across mixed DPI.
    let _ = panel.set_position(LogicalPosition::new(px, py));
    true
}

fn open_settings_window(app: &tauri::AppHandle) {
    // Opening Settings auto-dismisses the panel (gear button and tray menu):
    // the user is editing config, not choosing a resize target. Also stops
    // the target watcher, which exits when the panel hides.
    dismiss_panel(app);
    let window = match app.get_webview_window("settings") {
        Some(w) => w,
        None => {
            let built =
                WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
                    .title("Settings")
                    .theme(Some(tauri::Theme::Dark))
                    .inner_size(640.0, 700.0)
                    .min_inner_size(480.0, 420.0)
                    .build();
            match built {
                Ok(w) => w,
                Err(_) => return,
            }
        }
    };
    let _ = window.show();
    let _ = window.unminimize();
    // Activate the app so the window truly becomes key — panel dismissal
    // deactivates us, so a tray-opened Settings could otherwise show unfocused.
    #[cfg(target_os = "macos")]
    ax::activate_app();
    let _ = window.set_focus();
}

// ---------- setup ----------

fn seed_config(app: &tauri::AppHandle) -> Result<String, Box<dyn std::error::Error>> {
    let store = app.store(CONFIG_FILE)?;
    let version = store.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
    // Also reseed when the shape predates the namespaced sections (nothing
    // has shipped yet, so a pre-release store is simply reset, not migrated).
    // Markers: `keybindings` never existed in the flat shape, and the old
    // `grid` had no windowGap/screenMargins.
    let namespaced = store.get("keybindings").is_some()
        && store
            .get("grid")
            .is_some_and(|g| g.get("windowGap").is_some());
    if version != config::CONFIG_VERSION || !namespaced {
        // Reseed: clear first so keys from an older shape don't linger and
        // trip the schema's noUnknown validation.
        store.clear();
        for (k, v) in config::default_config_json()
            .as_object()
            .expect("default config is an object")
        {
            store.set(k, v.clone());
        }
        store.save()?;
    }
    Ok(store
        .get("keybindings")
        .and_then(|v| {
            v.get("openPanel")
                .and_then(|x| x.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| config::DEFAULT_HOTKEY.into()))
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Open", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &settings_item, &quit_item])?;

    // Left-click toggles the panel (via on_tray_icon_event below); the dropdown
    // menu is right-click-only. show_menu_on_left_click(false) suppresses the
    // menu on left-click — tray-icon's menu_on_right_click defaults to true, so
    // right-click still shows the dropdown.
    TrayIconBuilder::new()
        // Dedicated monochrome template glyph (silhouette of the app icon's
        // three-pane composition). The full-color app icon is full-bleed opaque,
        // which would render as a solid square in the menu bar — template images
        // draw from the alpha channel only.
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../assets/tray-template.png"
        ))?)
        .icon_as_template(true)
        .tooltip("Vindue")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => toggle_panel(app),
            "settings" => open_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_panel(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Registered first so plugin/setup errors land in the log too.
        // Default targets are stdout + the OS-standard log dir — macOS:
        // ~/Library/Logs/{bundle-id}/, Windows: %LOCALAPPDATA%\{bundle-id}\logs.
        // Info floor + quiet windowing libs: tao/wry TRACE-flood every
        // resize/move otherwise.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("tao", log::LevelFilter::Warn)
                .level_for("wry", log::LevelFilter::Warn)
                .build(),
        )
        // Second launch (Finder icon, `open`, login-item re-run) focuses the
        // running instance instead of spawning a twin that would fight over
        // the API port: Settings gets focus if open, otherwise the panel
        // opens exactly like the global hotkey would.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(settings) = app.get_webview_window("settings") {
                let _ = settings.set_focus();
            } else {
                toggle_panel(app);
            }
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_panel(app);
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        .manage(api::ApiServerState::default())
        .invoke_handler(tauri::generate_handler![
            ax_trusted,
            current_target,
            current_exe,
            apply_rect,
            dismiss,
            open_settings,
            commit_config,
            set_hotkey_enabled,
            config_stamp,
            reload_config,
            monitor_names,
            app_icon,
            running_apps,
            target_app,
            api::api_info
        ])
        .setup(|app| {
            // Menu-bar-only app: no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let hotkey = seed_config(app.handle())?;
            build_tray(app)?;
            // Pre-warm the first panel so the initial activation is instant;
            // extra panels (one per display) are created lazily on activation.
            let _ = ensure_panel(app.handle(), &format!("{PANEL_PREFIX}0"));

            if !hotkey.is_empty() {
                let shortcut: Shortcut = hotkey
                    .parse()
                    .map_err(|e| format!("Invalid hotkey \"{hotkey}\" in config: {e:?}"))?;
                app.global_shortcut().register(shortcut)?;
            }

            // Bring up the loopback HTTP/MCP server per the stored api config
            // (seed_config guarantees the key exists).
            let api_cfg = app
                .store(CONFIG_FILE)
                .ok()
                .and_then(|s| s.get("api"))
                .and_then(|v| serde_json::from_value::<config::ApiCfg>(v).ok())
                .unwrap_or(config::ApiCfg {
                    port: config::DEFAULT_API_PORT,
                    enabled: true,
                });
            api::apply_server_config(app.handle(), Some(api_cfg));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_mirrors_display_aspect_and_centers() {
        // 16" MacBook built-in: 1728×1117 logical, primary origin.
        let (pw, ph, px, py) = panel_placement(1728.0, 1117.0, 0.0, 0.0).unwrap();
        // Enough room => full grid height, and grid aspect == display aspect.
        assert_eq!(ph, GRID_H + CHROME_H);
        let grid_aspect = (pw - PAD_X) / (ph - CHROME_H);
        assert!((grid_aspect - 1728.0 / 1117.0).abs() < 1e-9);
        assert!((px - (1728.0 - pw) / 2.0).abs() < 1e-9);
        assert!((py - (1117.0 - ph) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn placement_clamps_extreme_aspect_ratios() {
        // Ultra-wide 5120×1080: display aspect 4.74 clamps the grid to 3.6.
        let (pw, ph, _, _) = panel_placement(5120.0, 1080.0, 0.0, 0.0).unwrap();
        assert!(((pw - PAD_X) / (ph - CHROME_H) - 3.6).abs() < 1e-9);
        // Portrait 800×1600: aspect 0.5 is in range, but the panel's
        // minimum width (340) floors the result.
        let (pw, _, _, _) = panel_placement(800.0, 1600.0, 0.0, 0.0).unwrap();
        assert_eq!(pw, 340.0);
    }

    #[test]
    fn placement_clamps_panel_height() {
        // Short display: height tracks work area minus breathing room.
        let (_, ph, _, _) = panel_placement(1920.0, 400.0, 0.0, 0.0).unwrap();
        assert_eq!(ph, 360.0);
        // Very short display: floor at 320.
        let (_, ph, _, _) = panel_placement(1920.0, 300.0, 0.0, 0.0).unwrap();
        assert_eq!(ph, 320.0);
    }

    #[test]
    fn placement_rejects_tiny_work_areas() {
        assert!(panel_placement(50.0, 50.0, 0.0, 0.0).is_none());
        assert!(panel_placement(1920.0, 90.0, 0.0, 0.0).is_none());
        assert!(panel_placement(90.0, 1080.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn placement_centers_on_secondary_display_origin() {
        // Work area at x=1728 (display to the right), y=-200 (above).
        let (pw, ph, px, py) = panel_placement(1440.0, 900.0, 1728.0, -200.0).unwrap();
        assert!((px - (1728.0 + (1440.0 - pw) / 2.0)).abs() < 1e-9);
        assert!((py - (-200.0 + (900.0 - ph) / 2.0)).abs() < 1e-9);
    }
}
