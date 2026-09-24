// Settings window: a form view (default) plus a raw-JSON view of config.json,
// both validated against the same Yup schema on save. External edits to the
// file are picked up automatically (file-stamp poll + store reload), so there
// is no Reload button.
import { useEffect, useRef, useState } from "react";
import { ValidationError } from "yup";
import { availableMonitors, getCurrentWindow, type Monitor } from "@tauri-apps/api/window";
import { appConfigDir, homeDir } from "@tauri-apps/api/path";
import { openPath } from "@tauri-apps/plugin-opener";
import { Columns3, GalleryHorizontal, GalleryVertical, Monitor as MonitorIcon, Rows3 } from "lucide-react";
import { validateConfig } from "./configSchema";
import { readRawConfig, type AppConfig, type PanelAssignment } from "./store";
import { apiInfo, commitConfig, configStamp, monitorNames, reloadConfig, type ApiInfo } from "./bindings";
import {
  rescaleSelection,
  selectionFromCells,
  type Cell,
  type Selection,
} from "./geometry";
import {
  friendlyMonitorName,
  monitorLabels,
  shortcutColor,
  SHORTCUT_KEYS,
  type MonitorRef,
  type ShortcutDef,
} from "./shortcuts";
import HotkeyInput from "./HotkeyInput";
import "./styles.css";

function toConfig(valid: Record<string, unknown>): AppConfig {
  return {
    grid: valid.grid as AppConfig["grid"],
    keybindings: valid.keybindings as AppConfig["keybindings"],
    shortcuts: valid.shortcuts as AppConfig["shortcuts"],
    api: valid.api as AppConfig["api"],
  };
}

function clampNum(s: string, min: number, max: number): number {
  const n = Number(s);
  if (Number.isNaN(n)) return min;
  return Math.min(max, Math.max(min, Math.round(n)));
}

/** Key-order-insensitive JSON fingerprint, for content comparisons. */
function sortDeep(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(sortDeep);
  if (v != null && typeof v === "object") {
    return Object.fromEntries(
      Object.entries(v as Record<string, unknown>)
        .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
        .map(([k, x]) => [k, sortDeep(x)]),
    );
  }
  return v;
}

function canonical(v: unknown): string {
  return JSON.stringify(sortDeep(v));
}

export default function Settings() {
  const [view, setView] = useState<"form" | "json">("form");
  const [version, setVersion] = useState(1);
  const [form, setForm] = useState<AppConfig | null>(null);
  const [text, setText] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const [savedHotkey, setSavedHotkey] = useState<string | undefined>();
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [nameMap, setNameMap] = useState<Map<number, string>>(new Map());
  const [pathAbs, setPathAbs] = useState<string | null>(null);
  const [pathDisplay, setPathDisplay] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [apiStatus, setApiStatus] = useState<ApiInfo | null>(null);
  const [picking, setPicking] = useState(false);
  const [drag, setDrag] = useState<{ key: string; anchor: Cell; focus: Cell } | null>(null);
  const dragRef = useRef(drag);
  dragRef.current = drag;

  // Auto-reload bookkeeping: fingerprint of the file as last seen, and the
  // canonical content as last loaded/saved (detects our own writes).
  const stampRef = useRef<string | null>(null);
  const savedCanonRef = useRef("");
  // Grid as last seen on disk — save-time best-effort rescale compares
  // against it to detect grid changes made in the JSON view.
  const diskGridRef = useRef<{ rows: number; cols: number } | null>(null);
  // Live refs for the poll closure (registered once).
  const viewRef = useRef(view);
  viewRef.current = view;
  const formRef = useRef(form);
  formRef.current = form;
  const textRef = useRef(text);
  textRef.current = text;
  const versionRef = useRef(version);
  versionRef.current = version;

  const applyContent = async (raw: Record<string, unknown>, str: string) => {
    savedCanonRef.current = canonical(raw);
    const g = raw.grid as { rows?: number; cols?: number } | undefined;
    diskGridRef.current =
      g && typeof g.rows === "number" && typeof g.cols === "number"
        ? { rows: g.rows, cols: g.cols }
        : null;
    const kb = raw.keybindings as { openPanel?: string } | undefined;
    setSavedHotkey(typeof kb?.openPanel === "string" ? kb.openPanel : undefined);
    setConflict(false);
    try {
      const valid = await validateConfig(raw);
      const wasBroken = formRef.current == null;
      setVersion(typeof raw.version === "number" ? raw.version : 1);
      setForm(toConfig(valid));
      setText(str);
      setErrors([]);
      setStatus(null);
      setPicking(false);
      setDrag(null);
      if (wasBroken) setView("form");
    } catch (e) {
      // Invalid config on disk: drop into the JSON view for manual repair.
      setForm(null);
      setText(str);
      setErrors(e instanceof ValidationError ? e.errors.map(String) : [String(e)]);
      setView("json");
    }
  };

  const refreshFromDisk = async () => {
    const stamp = await configStamp(); // fingerprint what we're about to read
    await reloadConfig();
    const raw = await readRawConfig();
    stampRef.current = stamp;
    await applyContent(raw, JSON.stringify(raw, null, 2));
  };

  useEffect(() => {
    refreshFromDisk().catch((e) => console.error("initial config load failed:", e));
    availableMonitors()
      .then(setMonitors)
      .catch((e) => console.error("availableMonitors failed:", e));
    monitorNames()
      .then((names) => setNameMap(new Map(names)))
      .catch((e) => console.error("monitorNames failed:", e));
    apiInfo()
      .then(setApiStatus)
      .catch((e) => console.error("apiInfo failed:", e));
    Promise.all([appConfigDir(), homeDir()])
      .then(([dir, home]) => {
        const abs = `${dir}config.json`;
        const h = home.endsWith("/") ? home : `${home}/`;
        setPathAbs(abs);
        setPathDisplay(abs.startsWith(h) ? `~/${abs.slice(h.length)}` : abs);
      })
      .catch((e) => console.error("config path resolution failed:", e));
  }, []);

  // Watch config.json for external edits (hand edits, or shortcut assignment
  // from the panel) and reload when our own view has no unsaved changes.
  useEffect(() => {
    const t = setInterval(async () => {
      try {
        const stamp = await configStamp();
        if (stamp == null || stamp === stampRef.current) return;
        await reloadConfig();
        const raw = await readRawConfig();
        const str = JSON.stringify(raw, null, 2);
        stampRef.current = stamp;
        const diskCanon = canonical(raw);
        if (diskCanon === savedCanonRef.current) return; // our own save
        const editingCanon = (() => {
          if (viewRef.current === "json") {
            try {
              return canonical(JSON.parse(textRef.current));
            } catch {
              return "unsaved";
            }
          }
          return formRef.current
            ? canonical({ version: versionRef.current, ...formRef.current })
            : "unsaved";
        })();
        if (editingCanon !== savedCanonRef.current) {
          setConflict(true); // unsaved edits — don't clobber them
          return;
        }
        await applyContent(raw, str);
      } catch {
        /* transient — next tick retries */
      }
    }, 1000);
    return () => clearInterval(t);
  }, []);

  // "Add shortcut" listens for the next assignable key press. New shortcuts
  // default to *Current display (follows panel)*; pin one per row if wanted.
  useEffect(() => {
    if (!picking) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setPicking(false);
        return;
      }
      if (e.metaKey || e.ctrlKey || !(e.code in SHORTCUT_KEYS)) return;
      e.preventDefault();
      setPicking(false);
      const cur = formRef.current;
      if (!cur) return;
      if (cur.shortcuts.keys[e.code]) {
        setErrors([`Key "${SHORTCUT_KEYS[e.code]}" is already assigned — remove it first`]);
        return;
      }
      const selection: Selection = {
        startRow: 0,
        endRow: cur.grid.rows - 1,
        startCol: 0,
        endCol: cur.grid.cols - 1,
      };
      setErrors([]);
      setForm({
        ...cur,
        shortcuts: {
          ...cur.shortcuts,
          keys: { ...cur.shortcuts.keys, [e.code]: { monitor: null, selection } },
        },
      });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [picking]);

  // Commit a mini-grid drag on release (anywhere in the window).
  useEffect(() => {
    const onUp = () => {
      const d = dragRef.current;
      setDrag(null);
      if (!d || !form) return;
      const def = form.shortcuts.keys[d.key];
      if (!def) return;
      const sel = selectionFromCells(d.anchor, d.focus);
      setForm({
        ...form,
        shortcuts: {
          ...form.shortcuts,
          keys: { ...form.shortcuts.keys, [d.key]: { ...def, selection: sel } },
        },
      });
    };
    window.addEventListener("mouseup", onUp);
    return () => window.removeEventListener("mouseup", onUp);
  });

  const switchJson = () => {
    if (view === "json") return;
    if (form) setText(JSON.stringify({ version, ...form }, null, 2));
    setErrors([]);
    setStatus(null);
    setView("json");
  };

  const switchForm = async () => {
    if (view === "form") return;
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch (e) {
      setErrors([`Invalid JSON: ${String(e)}`]);
      return;
    }
    let valid: Record<string, unknown>;
    try {
      valid = await validateConfig(parsed);
    } catch (e) {
      setErrors(e instanceof ValidationError ? e.errors.map(String) : [String(e)]);
      return;
    }
    setErrors([]);
    setVersion(typeof valid.version === "number" ? valid.version : version);
    setForm(toConfig(valid));
    setView("form");
  };

  const save = async () => {
    setStatus(null);
    let value: unknown;
    if (view === "form") {
      if (!form) return;
      value = { version, ...form };
    } else {
      try {
        value = JSON.parse(text);
      } catch (e) {
        setErrors([`Invalid JSON: ${String(e)}`]);
        return;
      }
    }

    // Best effort: if the grid changed vs. what's on disk (JSON-view edit),
    // proportionally remap any selections that no longer fit. Form-view
    // saves are a no-op here — patchGrid already rescaled them live.
    const dg = diskGridRef.current;
    const vg = (value as { grid?: { rows?: number; cols?: number } })?.grid;
    if (
      dg &&
      vg &&
      typeof vg.rows === "number" &&
      typeof vg.cols === "number" &&
      (dg.rows !== vg.rows || dg.cols !== vg.cols)
    ) {
      const sc = (value as { shortcuts?: { keys?: Record<string, ShortcutDef> } }).shortcuts;
      if (sc?.keys && typeof sc.keys === "object") {
        const to = { rows: vg.rows, cols: vg.cols };
        const remapped: Record<string, ShortcutDef> = {};
        let changed = false;
        for (const [k, def] of Object.entries(sc.keys)) {
          const s = def?.selection;
          if (s && (s.endRow >= to.rows || s.endCol >= to.cols)) {
            remapped[k] = { ...def, selection: rescaleSelection(s, dg, to) };
            changed = true;
          } else {
            remapped[k] = def;
          }
        }
        if (changed) {
          value = { ...(value as object), shortcuts: { ...sc, keys: remapped } };
        }
      }
    }

    let valid: Record<string, unknown>;
    try {
      valid = await validateConfig(value);
    } catch (e) {
      setErrors(e instanceof ValidationError ? e.errors.map(String) : [String(e)]);
      return;
    }
    setErrors([]);
    // One commit pipeline on the Rust side: validate → store write →
    // broadcast to open panels → hotkey swap → API-server rebind.
    try {
      await commitConfig(valid);
    } catch (err) {
      // The store was written and panels refreshed; only the hotkey swap
      // failed (conflict) — the previous hotkey remains registered.
      savedCanonRef.current = canonical(valid);
      diskGridRef.current = valid.grid as { rows: number; cols: number };
      setConflict(false);
      setForm(toConfig(valid));
      setText(JSON.stringify(valid, null, 2));
      setErrors([
        String(err),
        "All other settings were saved; the previous hotkey remains active.",
      ]);
      return;
    }
    savedCanonRef.current = canonical(valid);
    diskGridRef.current = valid.grid as { rows: number; cols: number };
    setConflict(false);
    setForm(toConfig(valid));
    setText(JSON.stringify(valid, null, 2));
    setSavedHotkey((valid.keybindings as { openPanel: string }).openPanel);
    // Changes apply immediately — a successful save closes the window.
    await getCurrentWindow().close();
  };

  const discardEdits = async () => {
    await refreshFromDisk();
    setStatus("Reloaded from disk.");
  };

  const copyPath = async () => {
    if (!pathAbs) return;
    try {
      await navigator.clipboard.writeText(pathAbs);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      console.error("clipboard write failed:", e);
    }
  };

  const openInEditor = async () => {
    if (!pathAbs) return;
    try {
      await openPath(pathAbs);
    } catch (e) {
      console.error("openPath(config.json) failed:", e);
    }
  };

  const copyMcpUrl = async () => {
    if (!form) return;
    try {
      await navigator.clipboard.writeText(`http://127.0.0.1:${form.api.port}/mcp`);
      setCopiedUrl(true);
      setTimeout(() => setCopiedUrl(false), 1500);
    } catch (e) {
      console.error("clipboard write failed:", e);
    }
  };

  // ----- form mutation helpers -----

  const patch = (p: Partial<AppConfig>) => form && setForm({ ...form, ...p });

  const patchGrid = (p: Partial<AppConfig["grid"]>) => {
    if (!form) return;
    const grid = { ...form.grid, ...p };
    if (grid.rows === form.grid.rows && grid.cols === form.grid.cols) {
      setForm({ ...form, grid });
      return;
    }
    // Best effort: remap saved selections proportionally onto the new grid,
    // so "left half" stays the left half.
    const keys: Record<string, ShortcutDef> = {};
    for (const [k, def] of Object.entries(form.shortcuts.keys)) {
      keys[k] = { ...def, selection: rescaleSelection(def.selection, form.grid, grid) };
    }
    setForm({ ...form, grid, shortcuts: { ...form.shortcuts, keys } });
  };

  const patchGap = (key: "width" | "height", n: number) => {
    if (!form) return;
    setForm({
      ...form,
      grid: { ...form.grid, windowGap: { ...form.grid.windowGap, [key]: n } },
    });
  };

  const patchEdgeMargin = (key: "top" | "right" | "bottom" | "left", n: number) => {
    if (!form) return;
    setForm({
      ...form,
      grid: { ...form.grid, screenMargins: { ...form.grid.screenMargins, [key]: n } },
    });
  };

  const setShortcutMonitor = (code: string, ref: MonitorRef | null) => {
    if (!form) return;
    const def = form.shortcuts.keys[code];
    if (!def) return;
    setForm({
      ...form,
      shortcuts: {
        ...form.shortcuts,
        keys: { ...form.shortcuts.keys, [code]: { ...def, monitor: ref } },
      },
    });
  };

  const removeShortcut = (code: string) => {
    if (!form) return;
    const rest = { ...form.shortcuts.keys };
    delete rest[code];
    setForm({ ...form, shortcuts: { ...form.shortcuts, keys: rest } });
  };

  // Canonical labels for the connected monitors — friendly names, with
  // positional suffixes disambiguating identical displays ("… (left)").
  const labels = monitorLabels(monitors, nameMap);

  const monitorLabel = (i: number) => labels[i] ?? `Display ${i + 1}`;

  const selectValue = (def: ShortcutDef): string => {
    if (def.monitor == null) return "current";
    if (def.monitor.name != null) {
      // Match the canonical label, or the raw tao name for entries saved
      // before friendly names existed ("Monitor #<id>").
      const byName = labels.indexOf(def.monitor.name);
      if (byName >= 0) return String(byName);
      const byRaw = monitors.findIndex((m) => m.name === def.monitor?.name);
      if (byRaw >= 0) return String(byRaw);
    }
    if (def.monitor.index < monitors.length) return String(def.monitor.index);
    return "missing";
  };

  const onSelectMonitor = (code: string) => (e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = e.target.value;
    if (v === "missing") return;
    if (v === "current") {
      setShortcutMonitor(code, null);
      return;
    }
    const i = Number(v);
    setShortcutMonitor(code, { name: monitorLabel(i), index: i });
  };

  const numField = (
    label: string,
    value: number,
    min: number,
    max: number,
    set: (n: number) => void,
    icon?: React.ReactNode,
  ) => (
    <label className="field">
      <span>
        {icon}
        {label}
      </span>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => set(clampNum(e.target.value, min, max))}
      />
    </label>
  );

  const edgeInput = (key: "top" | "right" | "bottom" | "left") => (
    <label className={`mw-edge mw-${key}`}>
      <input
        type="number"
        min={0}
        max={300}
        value={form?.grid.screenMargins[key] ?? 0}
        onChange={(e) => patchEdgeMargin(key, clampNum(e.target.value, 0, 300))}
        aria-label={`${key} screen margin`}
        title={`${key} screen margin (px)`}
      />
    </label>
  );

  const miniGrid = (code: string, def: ShortcutDef, color: string) => {
    if (!form) return null;
    const active =
      drag && drag.key === code ? selectionFromCells(drag.anchor, drag.focus) : def.selection;
    const cells = [];
    for (let r = 0; r < form.grid.rows; r++) {
      for (let c = 0; c < form.grid.cols; c++) {
        const inSel =
          r >= active.startRow &&
          r <= active.endRow &&
          c >= active.startCol &&
          c <= active.endCol;
        cells.push(
          <div
            key={`${r},${c}`}
            className={`mini-cell${inSel ? " sel" : ""}`}
            onMouseDown={(e) => {
              e.preventDefault();
              setDrag({
                key: code,
                anchor: { row: r, col: c },
                focus: { row: r, col: c },
              });
            }}
            onMouseEnter={() =>
              setDrag((d) => (d && d.key === code ? { ...d, focus: { row: r, col: c } } : d))
            }
          />,
        );
      }
    }
    return (
      <div
        className="mini-grid"
        style={
          {
            gridTemplateColumns: `repeat(${form.grid.cols}, 10px)`,
            gridTemplateRows: `repeat(${form.grid.rows}, 10px)`,
            "--chip": color,
          } as React.CSSProperties
        }
      >
        {cells}
      </div>
    );
  };

  return (
    <div className="settings">
      <header>
        <h1>Settings</h1>
        <div className="segmented">
          <button
            className={view === "form" ? "on" : ""}
            onClick={switchForm}
            disabled={form == null && view === "json"}
          >
            Form
          </button>
          <button className={view === "json" ? "on" : ""} onClick={switchJson}>
            JSON
          </button>
        </div>
      </header>

      {view === "json" && (
        <>
          <div className="config-path">
            <code title={pathAbs ?? undefined}>{pathDisplay ?? "…"}</code>
            <button className="copy" onClick={copyPath}>
              {copied ? "Copied" : "Copy"}
            </button>
            <button className="copy" onClick={openInEditor} disabled={!pathAbs}>
              Open
            </button>
          </div>
          <textarea
            spellCheck={false}
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
        </>
      )}

      {view === "form" && form && (
        <div className="form">
          <section className="settings-section">
            <div className="section-head">
              <h2>Grid</h2>
              <p className="hint">
                Rows and columns the panel divides each display into.
                Resizing remaps your saved shortcuts proportionally, so
                "left half" stays the left half. The panel mirrors each
                display's aspect ratio; changes apply immediately.
              </p>
            </div>
            <div className="section-body">
              <div className="row">
                {numField("Rows", form.grid.rows, 1, 12, (n) => patchGrid({ rows: n }), (
                  <Rows3 size={15} aria-hidden />
                ))}
                {numField("Columns", form.grid.cols, 1, 12, (n) => patchGrid({ cols: n }), (
                  <Columns3 size={15} aria-hidden />
                ))}
              </div>
              <span className="group-label">Window gap (px)</span>
              <div className="row">
                {numField("Horizontal", form.grid.windowGap.width, 0, 300, (n) =>
                  patchGap("width", n),
                (
                  <GalleryHorizontal size={15} aria-hidden />
                ))}
                {numField("Vertical", form.grid.windowGap.height, 0, 300, (n) =>
                  patchGap("height", n),
                (
                  <GalleryVertical size={15} aria-hidden />
                ))}
              </div>
              <span className="group-label">Screen margins (px)</span>
              <div className="margin-widget">
                {edgeInput("top")}
                {edgeInput("left")}
                <div className="mw-monitor" aria-hidden>
                  <MonitorIcon size={26} />
                </div>
                {edgeInput("right")}
                {edgeInput("bottom")}
              </div>
            </div>
          </section>

          <hr className="divider" />

          <section className="settings-section">
            <div className="section-head">
              <h2>Keybindings</h2>
              <p className="hint">
                The global hotkey toggles the panel. Click the field, then
                press a key combination to record it — at least one modifier
                is required. It re-registers live on Save; conflicts are
                detected best-effort, since macOS does not report all of
                them.
              </p>
            </div>
            <div className="section-body">
              <div className="field wide">
                <span>Open Vindue</span>
                <div className="hotkey-row">
                  <HotkeyInput
                    value={form.keybindings.openPanel}
                    onChange={(v) => patch({ keybindings: { openPanel: v } })}
                    registeredHotkey={savedHotkey ?? form.keybindings.openPanel}
                  />
                  {form.keybindings.openPanel && (
                    <button
                      className="copy"
                      title="Clear the hotkey — the panel stays reachable from the menu bar"
                      onClick={() => patch({ keybindings: { openPanel: "" } })}
                    >
                      ✕ Clear
                    </button>
                  )}
                </div>
              </div>
            </div>
          </section>

          <hr className="divider" />

          <section className="settings-section">
            <div className="section-head">
              <h2>Shortcuts</h2>
              <p className="hint">
                Assignable keys (digits and ` − = [ ] \ ; ' , . /) save grid
                regions. Press an assignable key while the panel is open to
                save its dragged region, or use Add below.
              </p>
            </div>
            <div className="section-body">
              <label className="field wide">
                <span>Panel assignments save as</span>
                <select
                  value={form.shortcuts.assignment}
                  onChange={(e) =>
                    setForm({
                      ...form,
                      shortcuts: {
                        ...form.shortcuts,
                        assignment: e.target.value as PanelAssignment,
                      },
                    })
                  }
                >
                  <option value="pinned">
                    Pinned — bind to the display it's created on
                  </option>
                  <option value="relative">
                    Relative — follows the panel at apply time
                  </option>
                </select>
              </label>
              {Object.keys(form.shortcuts.keys).length === 0 && (
                <p className="hint">
                  None yet — press an assignable key while the panel is open
                  to save its dragged region, or use Add below.
                </p>
              )}
              {Object.entries(form.shortcuts.keys).map(([code, def]) => {
                const color = shortcutColor(code, Object.keys(form.shortcuts.keys));
                const missing = selectValue(def) === "missing";
                return (
                  <div className="shortcut-row" key={code}>
                    <span className="chip" style={{ background: color }}>
                      {SHORTCUT_KEYS[code]}
                    </span>
                    <select value={selectValue(def)} onChange={onSelectMonitor(code)}>
                      <option value="current">Relative (follows panel)</option>
                      {monitors.map((_, i) => (
                        <option key={i} value={String(i)}>
                          {monitorLabel(i)}
                        </option>
                      ))}
                      {missing && (
                        <option value="missing">
                          ⚠{" "}
                          {friendlyMonitorName(def.monitor?.name ?? null, nameMap) ??
                            `Display ${(def.monitor?.index ?? 0) + 1}`}{" "}
                          (not connected)
                        </option>
                      )}
                    </select>
                    {miniGrid(code, def, color)}
                    <button className="del" title="Remove" onClick={() => removeShortcut(code)}>
                      ✕
                    </button>
                  </div>
                );
              })}
              <button
                className="add"
                onClick={() => {
                  setErrors([]);
                  setStatus(null);
                  setPicking(true);
                }}
                disabled={picking}
              >
                {picking ? "Press an assignable key… (esc cancels)" : "+ Add shortcut"}
              </button>
            </div>
          </section>

          <hr className="divider" />

          <section className="settings-section">
            <div className="section-head">
              <h2>API</h2>
              <p className="hint">
                Local control API for scripts (REST at /api/v1) and AI
                clients (MCP at /mcp). Listens on 127.0.0.1 only;
                browser-originated requests are rejected.
              </p>
            </div>
            <div className="section-body">
              <div className="row">
                <label className="field">
                  <span>Enabled</span>
                  <input
                    type="checkbox"
                    checked={form.api.enabled}
                    onChange={(e) =>
                      patch({ api: { ...form.api, enabled: e.target.checked } })
                    }
                  />
                </label>
                {numField("Port", form.api.port, 1024, 65535, (n) =>
                  patch({ api: { ...form.api, port: n } }),
                )}
              </div>
              <div className="field wide">
                <span>MCP endpoint</span>
                <div className="hotkey-row">
                  <code className="api-url">
                    http://127.0.0.1:{form.api.port}/mcp
                  </code>
                  <button className="copy" onClick={copyMcpUrl}>
                    {copiedUrl ? "Copied" : "Copy"}
                  </button>
                </div>
              </div>
              {apiStatus && (
                <p className="hint">
                  {apiStatus.running
                    ? `Listening on port ${apiStatus.boundPort}.`
                    : form.api.enabled
                      ? "Not running — Save applies changes."
                      : "Disabled."}
                </p>
              )}
            </div>
          </section>
        </div>
      )}

      {errors.length > 0 && (
        <ul className="errors">
          {errors.map((e, i) => (
            <li key={i}>{e}</li>
          ))}
        </ul>
      )}
      {conflict && (
        <div className="status warn">
          config.json changed on disk — Save will overwrite it, or discard your
          edits.
        </div>
      )}
      {status && !conflict && <div className="status">{status}</div>}

      <footer>
        {conflict && <button onClick={discardEdits}>Discard edits</button>}
        <button className="primary" onClick={save}>
          Save
        </button>
      </footer>
    </div>
  );
}
