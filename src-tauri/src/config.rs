// Config shape, defaults, validation, and the pure geometry/label ports.
//
// The top-level keys mirror the Settings UI sections 1:1: grid, keybindings,
// shortcuts, api. This module is the Rust twin of the TS side:
//   - defaults  <-> DEFAULT_CONFIG in src/store.ts (string-parity tests both ways)
//   - validate_config <-> configSchema.ts (fixtures/validate-cases.json, both suites)
//   - rescale_selection <-> rescaleSelection (fixtures/rescale-cases.json)
//   - monitor_labels <-> monitorLabels (fixtures/label-cases.json)
//   - selection_to_rect <-> selectionToRect (fixtures/rect-cases.json)
//   - preset_selection <-> presetSelection (fixtures/preset-cases.json)
// Any drift between the languages fails at least one test suite.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Config-schema revision. Prototyping stance: no migrations — when the
/// file's version differs, the store is cleared and reseeded with defaults.
pub const CONFIG_VERSION: i64 = 1;

/// Default global hotkey: none — the panel is always reachable from the
/// menu-bar tray; users opt into a hotkey via Settings.
pub const DEFAULT_HOTKEY: &str = "";

pub const DEFAULT_API_PORT: u16 = 47725;

// ---------- typed shape (serde, camelCase on the wire) ----------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub start_row: i64,
    pub end_row: i64,
    pub start_col: i64,
    pub end_col: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRef {
    pub name: Option<String>,
    pub index: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutDef {
    pub monitor: Option<MonitorRef>,
    pub selection: Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Assignment {
    Pinned,
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Gap {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Edges {
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
    pub left: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridCfg {
    pub rows: i64,
    pub cols: i64,
    pub window_gap: Gap,
    pub screen_margins: Edges,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeybindingsCfg {
    pub open_panel: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutsCfg {
    pub assignment: Assignment,
    pub keys: HashMap<String, ShortcutDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ApiCfg {
    pub port: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub grid: GridCfg,
    pub keybindings: KeybindingsCfg,
    pub shortcuts: ShortcutsCfg,
    pub api: ApiCfg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The canonical default config. Parity with the TS side (`DEFAULT_CONFIG`
/// in src/store.ts) is guarded from both directions:
/// `default_config_is_the_documented_shape` below pins these values, and
/// src/store.test.ts greps this function's literal against DEFAULT_CONFIG.
pub fn default_config_json() -> Value {
    json!({
        "version": CONFIG_VERSION,
        "grid": {
            "rows": 6, "cols": 6,
            "windowGap": { "width": 0, "height": 0 },
            "screenMargins": { "top": 0, "right": 0, "bottom": 0, "left": 0 }
        },
        "keybindings": { "openPanel": DEFAULT_HOTKEY },
        "shortcuts": { "assignment": "pinned", "keys": {} },
        "api": { "port": DEFAULT_API_PORT, "enabled": true }
    })
}

// ---------- validation (port of configSchema.ts; fixtures pin parity) ----------

/// Assignable keys — same set as SHORTCUT_KEYS in src/shortcuts.ts.
pub fn is_assignable_key(k: &str) -> bool {
    matches!(
        k,
        "Backquote"
            | "Digit1"
            | "Digit2"
            | "Digit3"
            | "Digit4"
            | "Digit5"
            | "Digit6"
            | "Digit7"
            | "Digit8"
            | "Digit9"
            | "Digit0"
            | "Minus"
            | "Equal"
            | "BracketLeft"
            | "BracketRight"
            | "Backslash"
            | "Semicolon"
            | "Quote"
            | "Comma"
            | "Period"
            | "Slash"
    )
}

fn int_in(v: &Value, min: i64, max: i64) -> bool {
    match v.as_i64() {
        // Reject floats even when integral (yup strict + .integer() does).
        Some(n) if v.is_i64() || v.is_u64() => n >= min && n <= max,
        _ => false,
    }
}

fn check_obj<'a>(
    v: &'a Value,
    expected: &[&str],
    path: &str,
    errs: &mut Vec<String>,
) -> Option<&'a serde_json::Map<String, Value>> {
    let Some(map) = v.as_object() else {
        errs.push(format!("{path} must be an object"));
        return None;
    };
    for k in map.keys() {
        if !expected.contains(&k.as_str()) {
            errs.push(format!("{path} has unknown key \"{k}\""));
        }
    }
    for k in expected {
        if !map.contains_key(*k) {
            errs.push(format!("{path} is missing \"{k}\""));
        }
    }
    Some(map)
}

fn px(v: Option<&Value>, path: &str, errs: &mut Vec<String>) {
    match v {
        Some(x) if int_in(x, 0, 300) => {}
        _ => errs.push(format!("{path} must be an integer 0-300")),
    }
}

/// Strict structural validation of a config.json value — the Rust twin of
/// validateConfig() in src/configSchema.ts. Collects all errors (like yup's
/// abortEarly: false). fixtures/validate-cases.json pins both to the same
/// accept/reject decisions.
pub fn validate_config(v: &Value) -> Result<(), Vec<String>> {
    let mut errs: Vec<String> = Vec::new();
    let Some(root) = v.as_object() else {
        return Err(vec!["config must be an object".into()]);
    };
    for k in root.keys() {
        if !matches!(
            k.as_str(),
            "version" | "grid" | "keybindings" | "shortcuts" | "api"
        ) {
            errs.push(format!("config has unknown key \"{k}\""));
        }
    }
    for k in ["version", "grid", "keybindings", "shortcuts", "api"] {
        if !root.contains_key(k) {
            errs.push(format!("config is missing \"{k}\""));
        }
    }
    if let Some(ver) = root.get("version") {
        if !int_in(ver, i64::MIN, i64::MAX) {
            errs.push("version must be an integer".into());
        }
    }

    // grid
    if let Some(g) = root.get("grid") {
        if let Some(m) = check_obj(
            g,
            &["rows", "cols", "windowGap", "screenMargins"],
            "grid",
            &mut errs,
        ) {
            for (k, lim) in [("rows", 12), ("cols", 12)] {
                match m.get(k) {
                    Some(x) if int_in(x, 1, lim) => {}
                    _ => errs.push(format!("grid.{k} must be an integer 1-{lim}")),
                }
            }
            if let Some(wg) = m.get("windowGap") {
                if let Some(wm) = check_obj(wg, &["width", "height"], "grid.windowGap", &mut errs) {
                    px(wm.get("width"), "grid.windowGap.width", &mut errs);
                    px(wm.get("height"), "grid.windowGap.height", &mut errs);
                }
            }
            if let Some(sm) = m.get("screenMargins") {
                if let Some(em) = check_obj(
                    sm,
                    &["top", "right", "bottom", "left"],
                    "grid.screenMargins",
                    &mut errs,
                ) {
                    for k in ["top", "right", "bottom", "left"] {
                        px(em.get(k), &format!("grid.screenMargins.{k}"), &mut errs);
                    }
                }
            }
        }
    }

    // keybindings
    if let Some(kb) = root.get("keybindings") {
        if let Some(m) = check_obj(kb, &["openPanel"], "keybindings", &mut errs) {
            match m.get("openPanel") {
                Some(Value::String(_)) => {}
                _ => errs.push("keybindings.openPanel must be a string".into()),
            }
        }
    }

    // shortcuts
    let mut grid_rc: Option<(i64, i64)> = None;
    if let Some(g) = root.get("grid").and_then(Value::as_object) {
        if let (Some(r), Some(c)) = (
            g.get("rows").and_then(Value::as_i64),
            g.get("cols").and_then(Value::as_i64),
        ) {
            grid_rc = Some((r, c));
        }
    }
    if let Some(sc) = root.get("shortcuts") {
        if let Some(m) = check_obj(sc, &["assignment", "keys"], "shortcuts", &mut errs) {
            match m.get("assignment").and_then(Value::as_str) {
                Some("pinned") | Some("relative") => {}
                _ => errs.push("shortcuts.assignment must be \"pinned\" or \"relative\"".into()),
            }
            match m.get("keys") {
                Some(Value::Object(keys)) => {
                    for (k, def) in keys {
                        if !is_assignable_key(k) {
                            errs.push(format!("shortcuts.keys: \"{k}\" is not an assignable key"));
                            continue;
                        }
                        let Some(dm) = check_obj(
                            def,
                            &["monitor", "selection"],
                            &format!("shortcuts.keys.{k}"),
                            &mut errs,
                        ) else {
                            continue;
                        };
                        match dm.get("monitor") {
                            Some(Value::Null) => {}
                            Some(mr) => {
                                if let Some(mm) = check_obj(
                                    mr,
                                    &["name", "index"],
                                    &format!("shortcuts.keys.{k}.monitor"),
                                    &mut errs,
                                ) {
                                    match mm.get("name") {
                                        Some(Value::String(_)) | Some(Value::Null) => {}
                                        _ => errs.push(format!(
                                            "shortcuts.keys.{k}.monitor.name must be a string or null"
                                        )),
                                    }
                                    if !int_in(mm.get("index").unwrap_or(&Value::Null), 0, i64::MAX)
                                    {
                                        errs.push(format!(
                                            "shortcuts.keys.{k}.monitor.index must be an integer >= 0"
                                        ));
                                    }
                                }
                            }
                            _ => errs.push(format!(
                                "shortcuts.keys.{k}.monitor must be an object or null"
                            )),
                        }
                        let Some(sel) = check_obj(
                            dm.get("selection").unwrap_or(&Value::Null),
                            &["startRow", "endRow", "startCol", "endCol"],
                            &format!("shortcuts.keys.{k}.selection"),
                            &mut errs,
                        ) else {
                            continue;
                        };
                        let get = |f: &str| sel.get(f).and_then(Value::as_i64);
                        let (sr, er, sc_, ec) = (
                            get("startRow"),
                            get("endRow"),
                            get("startCol"),
                            get("endCol"),
                        );
                        if let (Some(sr), Some(er), Some(sc_), Some(ec)) = (sr, er, sc_, ec) {
                            if sr < 0 || er < 0 || sc_ < 0 || ec < 0 {
                                errs.push(format!(
                                    "shortcuts.keys.{k}.selection fields must be integers >= 0"
                                ));
                            }
                            if sr > er || sc_ > ec {
                                errs.push(format!(
                                    "shortcuts.keys.{k}.selection start must be <= end"
                                ));
                            }
                            if let Some((rows, cols)) = grid_rc {
                                if er >= rows || ec >= cols {
                                    errs.push(format!(
                                        "shortcuts.keys.{k}.selection must fit within grid rows/cols"
                                    ));
                                }
                            }
                        } else {
                            errs.push(format!(
                                "shortcuts.keys.{k}.selection fields must be integers >= 0"
                            ));
                        }
                    }
                }
                Some(_) => errs.push("shortcuts.keys must be an object".into()),
                None => errs.push("shortcuts is missing \"keys\"".into()),
            }
        }
    }

    // api
    if let Some(a) = root.get("api") {
        if let Some(m) = check_obj(a, &["port", "enabled"], "api", &mut errs) {
            match m.get("port") {
                Some(p) if int_in(p, 1024, 65535) => {}
                _ => errs.push("api.port must be an integer 1024-65535".into()),
            }
            match m.get("enabled") {
                Some(Value::Bool(_)) => {}
                _ => errs.push("api.enabled must be a boolean".into()),
            }
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

// ---------- pure ports (fixtures pin parity with the TS originals) ----------

/// Port of rescaleSelection (src/geometry.ts). Rounding matches JS
/// Math.round for the non-negative values involved.
pub fn rescale_selection(sel: Selection, from: (i64, i64), to: (i64, i64)) -> Selection {
    fn span(start: i64, end: i64, old_n: i64, new_n: i64) -> (i64, i64) {
        let s = (((start * new_n) as f64 / old_n as f64).round() as i64).min(new_n - 1);
        let e = ((((end + 1) * new_n) as f64 / old_n as f64).round() as i64 - 1).max(s);
        (s, e)
    }
    let (start_row, end_row) = span(sel.start_row, sel.end_row, from.0, to.0);
    let (start_col, end_col) = span(sel.start_col, sel.end_col, from.1, to.1);
    Selection {
        start_row,
        end_row,
        start_col,
        end_col,
    }
}

/// Port of selectionToRect (src/geometry.ts). Physical pixels throughout.
pub fn selection_to_rect(sel: Selection, grid: &GridCfg, area: Rect) -> Rect {
    let inner = Rect {
        x: area.x + grid.screen_margins.left as f64,
        y: area.y + grid.screen_margins.top as f64,
        width: (area.width - grid.screen_margins.left as f64 - grid.screen_margins.right as f64)
            .max(0.0),
        height: (area.height - grid.screen_margins.top as f64 - grid.screen_margins.bottom as f64)
            .max(0.0),
    };
    let cell_w = inner.width / grid.cols as f64;
    let cell_h = inner.height / grid.rows as f64;
    let raw = Rect {
        x: inner.x + sel.start_col as f64 * cell_w,
        y: inner.y + sel.start_row as f64 * cell_h,
        width: (sel.end_col - sel.start_col + 1) as f64 * cell_w,
        height: (sel.end_row - sel.start_row + 1) as f64 * cell_h,
    };
    let inset_x = grid.window_gap.width as f64 / 2.0;
    let inset_y = grid.window_gap.height as f64 / 2.0;
    Rect {
        x: raw.x + inset_x,
        y: raw.y + inset_y,
        width: (raw.width - 2.0 * inset_x).max(0.0),
        height: (raw.height - 2.0 * inset_y).max(0.0),
    }
}

/// Port of presetSelection (src/geometry.ts).
pub fn preset_selection(name: &str, rows: i64, cols: i64) -> Option<Selection> {
    let half = |n: i64| ((n as f64 / 2.0).round() as i64).max(1);
    let (hr, hc) = (half(rows), half(cols));
    let sel = |sr, er, sc, ec| Selection {
        start_row: sr,
        end_row: er,
        start_col: sc,
        end_col: ec,
    };
    Some(match name {
        "full" => sel(0, rows - 1, 0, cols - 1),
        "left_half" => sel(0, rows - 1, 0, hc - 1),
        "right_half" => sel(0, rows - 1, cols - hc, cols - 1),
        "top_half" => sel(0, hr - 1, 0, cols - 1),
        "bottom_half" => sel(rows - hr, rows - 1, 0, cols - 1),
        "top_left" => sel(0, hr - 1, 0, hc - 1),
        "top_right" => sel(0, hr - 1, cols - hc, cols - 1),
        "bottom_left" => sel(rows - hr, rows - 1, 0, hc - 1),
        "bottom_right" => sel(rows - hr, rows - 1, cols - hc, cols - 1),
        _ => return None,
    })
}

/// A monitor for labeling purposes: raw tao name + global position.
pub struct LabelMon {
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
}

/// tao names macOS monitors "Monitor #<CGDisplayModelNumber>"; extract it.
/// Port of displayIdFromName (src/shortcuts.ts).
pub fn display_id_from_name(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("Monitor #")?;
    rest.parse::<u32>().ok()
}

/// Port of friendlyMonitorName (src/shortcuts.ts).
pub fn friendly_monitor_name(name: Option<&str>, names: &HashMap<u32, String>) -> Option<String> {
    let n = name?;
    if let Some(id) = display_id_from_name(n) {
        if let Some(f) = names.get(&id) {
            return Some(f.clone());
        }
    }
    Some(n.to_string())
}

/// Port of monitorLabels (src/shortcuts.ts): friendly names, with positional
/// suffixes disambiguating identical-name displays ((left)/(right)/(middle)/
/// (left-to-right #N)), duplicate groups ordered by (x, y).
pub fn monitor_labels(monitors: &[LabelMon], names: &HashMap<u32, String>) -> Vec<String> {
    let base: Vec<String> = monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            friendly_monitor_name(m.name.as_deref(), names)
                .unwrap_or_else(|| format!("Display {}", i + 1))
        })
        .collect();
    let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, b) in base.iter().enumerate() {
        groups.entry(b.as_str()).or_default().push(i);
    }
    let mut labels = base.clone();
    for (_b, idxs) in groups {
        if idxs.len() < 2 {
            continue;
        }
        let mut sorted = idxs.clone();
        // Stable sort by (x, y) — same ordering as the TS localeCompare-free
        // numeric comparator.
        sorted
            .sort_by(|&a, &c| (monitors[a].x, monitors[a].y).cmp(&(monitors[c].x, monitors[c].y)));
        let n = sorted.len();
        for (rank, &mi) in sorted.iter().enumerate() {
            let word = match n {
                2 => {
                    if rank == 0 {
                        "left"
                    } else {
                        "right"
                    }
                }
                3 => ["left", "middle", "right"][rank],
                _ => "",
            };
            labels[mi] = if word.is_empty() {
                format!("{} (left-to-right #{})", base[mi], rank + 1)
            } else {
                format!("{} ({word})", base[mi])
            };
        }
    }
    labels
}

/// Strip a positional disambiguation suffix — port of SUFFIX_RE handling in
/// resolveMonitorIndex (src/shortcuts.ts).
pub fn strip_label_suffix(label: &str) -> &str {
    let Some(open) = label.rfind(" (") else {
        return label;
    };
    if !label.ends_with(')') {
        return label;
    }
    let inner = &label[open + 2..label.len() - 1];
    let ok = matches!(inner, "left" | "middle" | "right")
        || inner
            .strip_prefix("left-to-right #")
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if ok {
        &label[..open]
    } else {
        label
    }
}

/// Port of resolveMonitorIndex (src/shortcuts.ts). `fallback` is the index
/// used when nothing matches; `fell_back` flags inexact resolutions.
/// Runtime callers live on the TS side; this port is exercised by the
/// parity fixture tests (fixtures/).
#[allow(dead_code)]
pub fn resolve_monitor_index(
    ref_name: Option<&str>,
    ref_index: i64,
    has_ref: bool,
    monitors: &[LabelMon],
    names: &HashMap<u32, String>,
    fallback: usize,
) -> (usize, bool) {
    if !has_ref {
        return (fallback, false);
    }
    if let Some(name) = ref_name {
        let labels = monitor_labels(monitors, names);
        if let Some(i) = labels.iter().position(|l| l == name) {
            return (i, false);
        }
        let bare = strip_label_suffix(name);
        if bare != name {
            if let Some(i) = labels.iter().position(|l| l == bare) {
                return (i, false);
            }
        }
        if let Some(i) = monitors
            .iter()
            .position(|m| m.name.as_deref() == Some(name))
        {
            return (i, false);
        }
        if ref_index >= 0 && (ref_index as usize) < monitors.len() {
            return (ref_index as usize, true);
        }
        return (fallback, true);
    }
    if ref_index >= 0 && (ref_index as usize) < monitors.len() {
        return (ref_index as usize, false);
    }
    (fallback, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_the_documented_shape() {
        // Pins the seeded defaults; src/store.test.ts pins the other side.
        let d = default_config_json();
        assert_eq!(d["version"], json!(CONFIG_VERSION));
        assert_eq!(d["grid"]["rows"], json!(6));
        assert_eq!(d["grid"]["cols"], json!(6));
        assert_eq!(d["grid"]["windowGap"], json!({ "width": 0, "height": 0 }));
        assert_eq!(
            d["grid"]["screenMargins"],
            json!({ "top": 0, "right": 0, "bottom": 0, "left": 0 })
        );
        assert_eq!(d["keybindings"]["openPanel"], json!(""));
        assert_eq!(d["shortcuts"]["assignment"], json!("pinned"));
        assert_eq!(d["shortcuts"]["keys"], json!({}));
        assert_eq!(
            d["api"],
            json!({ "port": DEFAULT_API_PORT, "enabled": true })
        );
    }

    #[test]
    fn default_config_validates() {
        assert!(validate_config(&default_config_json()).is_ok());
    }

    #[test]
    fn rescale_matches_known_vectors() {
        let s = rescale_selection(
            Selection {
                start_row: 0,
                end_row: 5,
                start_col: 0,
                end_col: 2,
            },
            (6, 6),
            (8, 8),
        );
        assert_eq!(
            s,
            Selection {
                start_row: 0,
                end_row: 7,
                start_col: 0,
                end_col: 3
            }
        );
    }

    #[test]
    fn labels_disambiguate_twins() {
        let names = HashMap::from([(100u32, "DELL".to_string())]);
        let mons = vec![
            LabelMon {
                name: Some("Monitor #100".into()),
                x: 2560,
                y: 0,
            },
            LabelMon {
                name: Some("Monitor #100".into()),
                x: 0,
                y: 0,
            },
        ];
        assert_eq!(
            monitor_labels(&mons, &names),
            vec!["DELL (right)", "DELL (left)"]
        );
    }

    #[test]
    fn suffix_strip() {
        assert_eq!(strip_label_suffix("DELL (left)"), "DELL");
        assert_eq!(strip_label_suffix("DELL (left-to-right #3)"), "DELL");
        assert_eq!(strip_label_suffix("DELL (27in)"), "DELL (27in)");
        assert_eq!(strip_label_suffix("DELL"), "DELL");
    }

    // ---------- golden-vector fixtures (shared with src/fixtures.test.ts) ----------
    //
    // Every case in fixtures/*.json runs through both the TS implementations
    // and these ports; the fixtures are the contract between the two.

    fn fixture_cases(json: &str) -> Vec<Value> {
        let v: Value = serde_json::from_str(json).expect("fixture parses");
        v["cases"].as_array().expect("cases array").clone()
    }

    fn rect_from(v: &Value) -> Rect {
        Rect {
            x: v["x"].as_f64().unwrap(),
            y: v["y"].as_f64().unwrap(),
            width: v["width"].as_f64().unwrap(),
            height: v["height"].as_f64().unwrap(),
        }
    }

    fn sel_from(v: &Value) -> Selection {
        serde_json::from_value(v.clone()).expect("selection parses")
    }

    fn mons_from(v: &Value) -> Vec<LabelMon> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|m| LabelMon {
                name: m["name"].as_str().map(String::from),
                x: m["x"].as_i64().unwrap() as i32,
                y: m["y"].as_i64().unwrap() as i32,
            })
            .collect()
    }

    fn names_from(v: &Value) -> HashMap<u32, String> {
        v.as_object()
            .unwrap()
            .iter()
            .map(|(k, n)| (k.parse().unwrap(), n.as_str().unwrap().to_string()))
            .collect()
    }

    #[test]
    fn validate_fixtures_pinned() {
        for case in fixture_cases(include_str!("../../fixtures/validate-cases.json")) {
            let result = validate_config(&case["config"]);
            let want_valid = case["valid"].as_bool().unwrap();
            assert_eq!(
                result.is_ok(),
                want_valid,
                "{}: {:?}",
                case["name"],
                result.err()
            );
        }
    }

    #[test]
    fn rescale_fixtures_pinned() {
        for case in fixture_cases(include_str!("../../fixtures/rescale-cases.json")) {
            let from = (
                case["from"]["rows"].as_i64().unwrap(),
                case["from"]["cols"].as_i64().unwrap(),
            );
            let to = (
                case["to"]["rows"].as_i64().unwrap(),
                case["to"]["cols"].as_i64().unwrap(),
            );
            let got = rescale_selection(sel_from(&case["sel"]), from, to);
            assert_eq!(got, sel_from(&case["expect"]), "{}", case["name"]);
        }
    }

    #[test]
    fn rect_fixtures_pinned() {
        for case in fixture_cases(include_str!("../../fixtures/rect-cases.json")) {
            let grid: GridCfg = serde_json::from_value(case["grid"].clone()).unwrap();
            let got = selection_to_rect(sel_from(&case["sel"]), &grid, rect_from(&case["area"]));
            let want = rect_from(&case["expect"]);
            let name = case["name"].as_str().unwrap();
            for (g, w) in [
                (got.x, want.x),
                (got.y, want.y),
                (got.width, want.width),
                (got.height, want.height),
            ] {
                assert!((g - w).abs() < 1e-9, "{name}: {got:?} != {want:?}");
            }
        }
    }

    #[test]
    fn preset_fixtures_pinned() {
        for case in fixture_cases(include_str!("../../fixtures/preset-cases.json")) {
            let got = preset_selection(
                case["preset"].as_str().unwrap(),
                case["grid"]["rows"].as_i64().unwrap(),
                case["grid"]["cols"].as_i64().unwrap(),
            );
            let want = if case["expect"].is_null() {
                None
            } else {
                Some(sel_from(&case["expect"]))
            };
            assert_eq!(got, want, "{}", case["name"]);
        }
    }

    #[test]
    fn label_fixtures_pinned() {
        for case in fixture_cases(include_str!("../../fixtures/label-cases.json")) {
            let got = monitor_labels(&mons_from(&case["monitors"]), &names_from(&case["names"]));
            let want: Vec<String> = case["expect"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            assert_eq!(got, want, "{}", case["name"]);
        }
    }

    #[test]
    fn resolve_fixtures_pinned() {
        for case in fixture_cases(include_str!("../../fixtures/resolve-cases.json")) {
            let has_ref = !case["ref"].is_null();
            let ref_name = case["ref"]["name"].as_str();
            let ref_index = case["ref"]["index"].as_i64().unwrap_or(0);
            let (idx, fell) = resolve_monitor_index(
                ref_name,
                ref_index,
                has_ref,
                &mons_from(&case["monitors"]),
                &names_from(&case["names"]),
                case["fallback"].as_u64().unwrap() as usize,
            );
            assert_eq!(
                (idx, fell),
                (
                    case["expect"]["index"].as_u64().unwrap() as usize,
                    case["expect"]["fellBack"].as_bool().unwrap()
                ),
                "{}",
                case["name"]
            );
        }
    }
}
