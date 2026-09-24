import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { availableMonitors, currentMonitor, type Monitor } from "@tauri-apps/api/window";
import {
  applyRect,
  appIcon,
  axTrusted,
  commitConfig,
  currentExe,
  currentTarget,
  dismissPanel,
  monitorNames,
  openSettings,
  runningApps,
  targetApp,
  type Target,
} from "./bindings";
import { loadConfig, saveShortcut, type AppConfig } from "./store";
import {
  selectionFromCells,
  selectionToRect,
  type Cell,
  type Selection,
} from "./geometry";
import {
  monitorLabels,
  resolveMonitorIndex,
  shortcutColor,
  SHORTCUT_KEYS,
  type MonitorRef,
  type ShortcutDef,
} from "./shortcuts";
import { Trash2 } from "lucide-react";
import "./styles.css";

function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [target, setTarget] = useState<Target | null>(null);
  const [iconUrls, setIconUrls] = useState<Map<number, string>>(new Map());
  const [pickerOpen, setPickerOpen] = useState(false);
  const [apps, setApps] = useState<{ pid: number; name: string }[]>([]);
  const [axOk, setAxOk] = useState<boolean | null>(null);
  const [exePath, setExePath] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [nameMap, setNameMap] = useState<Map<number, string>>(new Map());
  const [curIdx, setCurIdx] = useState(0);
  const [anchor, setAnchor] = useState<Cell | null>(null);
  const [focusCell, setFocusCell] = useState<Cell | null>(null);
  const [assigning, setAssigning] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const draggingRef = useRef(false);
  const assigningRef = useRef<string | null>(null);

  const setAssigningBoth = (v: string | null) => {
    assigningRef.current = v;
    setAssigning(v);
  };

  const clampIdx = () =>
    monitors.length === 0 ? 0 : Math.min(Math.max(curIdx, 0), monitors.length - 1);

  const refreshMonitors = async () => {
    try {
      const [all, cur] = await Promise.all([availableMonitors(), currentMonitor()]);
      setMonitors(all);
      if (cur) {
        const idx = all.findIndex(
          (m) => m.position.x === cur.position.x && m.position.y === cur.position.y,
        );
        setCurIdx(idx >= 0 ? idx : 0);
      }
    } catch (e) {
      // Keep the previous monitor list; log so failures are inspectable.
      console.error("monitor refresh failed:", e);
    }
    // Separate from the list fetch: a names failure must not blank monitors.
    monitorNames()
      .then((names) => setNameMap(new Map(names)))
      .catch((e) => console.error("monitorNames failed:", e));
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let unlistenTarget: (() => void) | undefined;
    let unlistenSaved: (() => void) | undefined;
    loadConfig().then(setConfig);
    currentTarget().then(setTarget);
    currentExe().then(setExePath);
    axTrusted(false).then(setAxOk);
    refreshMonitors();

    listen<Target | null>("panel-activated", (e) => {
      setTarget(e.payload);
      setError(null);
      setPreview(null);
      setAssigningBoth(null);
      loadConfig().then(setConfig);
      axTrusted(false).then(setAxOk);
      refreshMonitors();
    }).then((f) => (unlisten = f));

    // Live retargeting while the panel is open: the Rust watcher tracks the
    // frontmost window and pushes updates (window switch, move, resize).
    listen<Target>("target-changed", (e) => {
      setTarget(e.payload);
    }).then((f) => (unlistenTarget = f));

    // Settings saved while this panel is open: pick the config up live.
    listen("config-saved", () => {
      setPreview(null);
      loadConfig().then(setConfig);
    }).then((f) => (unlistenSaved = f));

    return () => {
      unlisten?.();
      unlistenTarget?.();
      unlistenSaved?.();
    };
  }, []);

  // Fetch (once per pid) and cache an app icon for the header/picker.
  const fetchIcon = (pid: number) => {
    if (iconUrls.has(pid)) return;
    appIcon(pid)
      .then((buf) => {
        if (buf.byteLength === 0) return;
        const url = URL.createObjectURL(new Blob([buf], { type: "image/png" }));
        setIconUrls((prev) => (prev.has(pid) ? prev : new Map(prev).set(pid, url)));
      })
      .catch((e) => console.error("appIcon failed:", e));
  };

  useEffect(() => {
    if (target == null) return;
    fetchIcon(target.pid);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target?.pid]);

  // Header app-picker: deliberate retargeting (the panels themselves are
  // true modals — clicking any other app's window dismisses them).
  const openPicker = async () => {
    setPickerOpen(true);
    try {
      const list = await runningApps();
      const parsed = list.map(([pid, name]) => ({ pid, name }));
      setApps(parsed);
      for (const a of parsed) fetchIcon(a.pid);
    } catch (e) {
      console.error("runningApps failed:", e);
    }
  };

  const pickApp = async (pid: number) => {
    setPickerOpen(false);
    try {
      const t = await targetApp(pid);
      if (t) setTarget(t);
    } catch (e) {
      console.error("targetApp failed:", e);
    }
  };

  // Auto-detect the Accessibility grant: poll while the banner is up so the
  // panel becomes usable the moment access is enabled in System Settings.
  useEffect(() => {
    if (axOk !== false) return;
    const timer = setInterval(() => {
      axTrusted(false).then((ok) => {
        if (ok) setAxOk(true);
      });
    }, 750);
    return () => clearInterval(timer);
  }, [axOk]);

  const rows = config?.grid.rows ?? 6;
  const cols = config?.grid.cols ?? 6;

  const selection = (): Selection | null => {
    if (!anchor || !focusCell) return null;
    return selectionFromCells(anchor, focusCell);
  };

  const isHighlighted = (c: Cell): boolean => {
    const sel = selection();
    return (
      sel != null &&
      c.row >= sel.startRow &&
      c.row <= sel.endRow &&
      c.col >= sel.startCol &&
      c.col <= sel.endCol
    );
  };

  /** Move the target window per a shortcut def; false (+ error) on failure. */
  const applyRectFor = async (def: ShortcutDef): Promise<boolean> => {
    if (!config || monitors.length === 0) return false;
    const { index } = resolveMonitorIndex(def.monitor, monitors, clampIdx(), nameMap);
    const monitor = monitors[index];
    const rect = selectionToRect(def.selection, config.grid, {
      x: monitor.workArea.position.x,
      y: monitor.workArea.position.y,
      width: monitor.workArea.size.width,
      height: monitor.workArea.size.height,
    });
    try {
      await applyRect(rect, monitor.scaleFactor);
      return true;
    } catch (err) {
      const msg = String(err);
      setError(msg);
      // The cached grant can go stale mid-session (a rebuild mints a new
      // code-signing identity). If the failure is lost access, drop axOk so the
      // banner reappears and the grant-polling effect resumes — recovering
      // without a panel re-activation.
      if (msg.includes("kAXErrorAPIDisabled")) setAxOk(false);
      return false;
    }
  };

  /** Apply a shortcut definition on its resolved monitor, then close. */
  const applyDef = async (def: ShortcutDef) => {
    if (await applyRectFor(def)) await dismissPanel();
  };

  const finalize = async () => {
    draggingRef.current = false;
    const sel = selection();
    setAnchor(null);
    setFocusCell(null);
    if (!sel || !config) return;

    const code = assigningRef.current;
    if (code) {
      // Assignment mode: bind this key to the dragged region *on this display*.
      setAssigningBoth(null);
      const cur = clampIdx();
      const m = monitors[cur];
      // "pinned" binds the shortcut to this display; "relative" saves no
      // monitor — the shortcut follows the panel at apply time.
      const ref: MonitorRef | null =
        m && config.shortcuts.assignment === "pinned"
          ? { name: monitorLabels(monitors, nameMap)[cur] ?? null, index: cur }
          : null;
      const def: ShortcutDef = { monitor: ref, selection: sel };
      try {
        await saveShortcut(code, def);
        setConfig({
          ...config,
          shortcuts: {
            ...config.shortcuts,
            keys: { ...config.shortcuts.keys, [code]: def },
          },
        });
      } catch (err) {
        setError(String(err));
        return;
      }
      // One motion: save, apply, close. On failure (e.g. no Accessibility
      // grant) applyDef leaves the panel open with the error, and the fresh
      // chip is proof the save landed.
      await applyDef(def);
      return;
    }
    await applyDef({ monitor: null, selection: sel });
  };

  /** Quick-delete a shortcut from its chip's hover trash affordance. */
  const deleteShortcut = async (code: string) => {
    if (!config) return;
    const keys = { ...config.shortcuts.keys };
    delete keys[code];
    const next = { ...config, shortcuts: { ...config.shortcuts, keys } };
    try {
      await commitConfig({ version: 1, ...next });
      setConfig(next);
      setPreview((p) => (p === code ? null : p));
    } catch (err) {
      setError(String(err));
    }
  };

  // Keyboard: Esc dismiss/cancel-assign; assignable keys apply (or start
  // assignment for) grid shortcuts. Physical e.code is used because macOS
  // Option/Shift change e.key (Option+1 => "!", Shift+` => "~").
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (assigningRef.current) setAssigningBoth(null);
        else dismissPanel();
        return;
      }
      if (e.metaKey || e.ctrlKey || !config || !(e.code in SHORTCUT_KEYS)) return;
      e.preventDefault();
      const code = e.code;
      const def = config.shortcuts.keys[code];
      if (def && !e.altKey) {
        applyDef(def);
      } else {
        setAssigningBoth(code);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // Release anywhere in the panel ends the drag (e.g. mouse up over the header).
  useEffect(() => {
    const onUp = () => {
      if (draggingRef.current) finalize();
    };
    window.addEventListener("mouseup", onUp);
    return () => window.removeEventListener("mouseup", onUp);
  });

  const onCellDown = (c: Cell) => (e: React.MouseEvent) => {
    e.preventDefault();
    setPreview(null);
    draggingRef.current = true;
    setAnchor(c);
    setFocusCell(c);
  };

  const onCellEnter = (c: Cell) => () => {
    if (draggingRef.current) setFocusCell(c);
  };

  const onCellUp = () => {
    if (draggingRef.current) finalize();
  };

  // Chips for shortcuts that resolve to THIS display go on their anchor cell;
  // the rest are summarized in the footer (they still fire — the window flies
  // to its saved display).
  const { byCell, others } = useMemo(() => {
    const byCell = new Map<string, { code: string; color: string }[]>();
    const others: { code: string; color: string; label: string }[] = [];
    const codes = Object.keys(config?.shortcuts.keys ?? {});
    const cur =
      monitors.length === 0 ? 0 : Math.min(Math.max(curIdx, 0), monitors.length - 1);
    for (const code of codes) {
      const def = config!.shortcuts.keys[code];
      const color = shortcutColor(code, codes);
      const { index } = resolveMonitorIndex(def.monitor, monitors, cur, nameMap);
      if (monitors.length === 0 || index === cur) {
        const k = `${def.selection.startRow},${def.selection.startCol}`;
        byCell.set(k, [...(byCell.get(k) ?? []), { code, color }]);
      } else {
        others.push({
          code,
          color,
          label: monitorLabels(monitors, nameMap)[index] ?? `Display ${index + 1}`,
        });
      }
    }
    return { byCell, others };
  }, [config, monitors, curIdx, nameMap]);

  const previewSel = preview ? config?.shortcuts.keys[preview]?.selection ?? null : null;
  const previewColor = preview
    ? shortcutColor(preview, Object.keys(config?.shortcuts.keys ?? {}))
    : undefined;
  const gridStyle = previewColor
    ? ({ "--pv": previewColor } as React.CSSProperties)
    : undefined;

  // A selection is live (drag in progress) — the moment to nudge that any
  // key press right now would save this region as a shortcut.
  const activeSel = selection() != null;

  // The file the user must grant: the .app bundle when bundled, otherwise the
  // raw (dev) binary. Ad-hoc signed builds change identity every rebuild, so
  // stale System Settings entries must be removed and this exact file re-added.
  const grantTarget = useMemo(() => {
    if (!exePath) return null;
    const i = exePath.indexOf(".app/");
    return i >= 0 ? exePath.slice(0, i + ".app".length) : exePath;
  }, [exePath]);

  const copyPath = async () => {
    if (!grantTarget) return;
    try {
      await navigator.clipboard.writeText(grantTarget);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      console.error("clipboard write failed:", e);
    }
  };

  return (
    <div className="panel">
      <header>
        <button
          className="close"
          title="Dismiss (Esc)"
          onClick={() => dismissPanel()}
        >
          ✕
        </button>
        <button
          className="target"
          title="Choose target app"
          onClick={() => (pickerOpen ? setPickerOpen(false) : openPicker())}
        >
          {target && iconUrls.get(target.pid) && (
            <img className="app-icon" src={iconUrls.get(target.pid)} alt="" />
          )}
          <span className="name">
            {target ? target.app_name || `pid ${target.pid}` : "no target window"}
          </span>
          <span className="caret">▾</span>
        </button>
        {pickerOpen && (
          <div className="app-picker">
            {apps.map((a) => (
              <button
                key={a.pid}
                className={a.pid === target?.pid ? "on" : ""}
                onClick={() => pickApp(a.pid)}
              >
                {iconUrls.get(a.pid) && (
                  <img className="app-icon" src={iconUrls.get(a.pid)} alt="" />
                )}
                {a.name}
              </button>
            ))}
            {apps.length === 0 && <div className="hint">No apps found</div>}
          </div>
        )}
        <button
          className="gear"
          title="Settings"
          onClick={() => openSettings()}
        >
          ⚙
        </button>
      </header>

      <div
        className="grid"
        onClick={() => setPickerOpen(false)}
        style={{
          gridTemplateColumns: `repeat(${cols}, 1fr)`,
          gridTemplateRows: `repeat(${rows}, 1fr)`,
          ...gridStyle,
        }}
      >
        {Array.from({ length: rows * cols }, (_, i) => {
          const c: Cell = { row: Math.floor(i / cols), col: i % cols };
          const cellChips = byCell.get(`${c.row},${c.col}`) ?? [];
          const sel = isHighlighted(c);
          return (
            <div
              key={i}
              className={`cell${sel ? " sel" : ""}`}
              // Diagonal shimmer: stagger the breathing loop by (row + col)
              // so the glow sweeps across the selection while it's held.
              style={
                sel
                  ? { animationDelay: `0s, ${160 + (c.row + c.col) * 55}ms` }
                  : undefined
              }
              onMouseDown={onCellDown(c)}
              onMouseEnter={onCellEnter(c)}
              onMouseUp={onCellUp}
            >
              {cellChips.length > 0 && (
                <div className="badges">
                  {cellChips.map((ch) => (
                    <span
                      key={ch.code}
                      className="badge"
                      style={{ background: ch.color }}
                      onMouseEnter={() => setPreview(ch.code)}
                      onMouseLeave={() => setPreview(null)}
                    >
                      {SHORTCUT_KEYS[ch.code]}
                      <button
                        className="trash"
                        title={`Delete shortcut ${SHORTCUT_KEYS[ch.code]}`}
                        onMouseDown={(e) => {
                          // Never start a cell drag from the trash button.
                          e.stopPropagation();
                          e.preventDefault();
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteShortcut(ch.code);
                        }}
                      >
                        <Trash2 size={12} strokeWidth={2.5} />
                      </button>
                    </span>
                  ))}
                </div>
              )}
            </div>
          );
        })}
        {/* Preview outline: one overlay spanning the region, so the dashed
            border is continuous instead of chopped by the cell gaps. */}
        {previewSel && (
          <div
            className="pv-outline"
            style={{
              gridRow: `${previewSel.startRow + 1} / ${previewSel.endRow + 2}`,
              gridColumn: `${previewSel.startCol + 1} / ${previewSel.endCol + 2}`,
            }}
          />
        )}
      </div>

      <footer>
        <div className="status">
          {assigning ? (
            <span className="assigning">
              assigning{" "}
              <span
                className="badge inline"
                style={{
                  background: shortcutColor(
                    assigning,
                    Object.keys(config?.shortcuts.keys ?? {}),
                  ),
                }}
              >
                {SHORTCUT_KEYS[assigning]}
              </span>{" "}
              — drag a region on this display · release saves & applies (esc
              cancels)
            </span>
          ) : activeSel ? (
            <span className="nudge">
              press a key now to save this region as a shortcut
              <span className="then">· release saves & applies</span>
            </span>
          ) : error ? (
            <span className="error">{error}</span>
          ) : (
            <span className="hint">
              drag to select · keys apply shortcuts (⌥+key reassigns) · esc
              dismisses
            </span>
          )}
        </div>
        {others.length > 0 && (
          <div className="others">
            <span className="hint">other displays:</span>
            {others.map((o) => (
              <span key={o.code} className="other">
                <span className="badge inline" style={{ background: o.color }}>
                  {SHORTCUT_KEYS[o.code]}
                </span>
                {o.label}
              </span>
            ))}
          </div>
        )}
      </footer>

      {axOk === false && (
        <div className="ax-banner">
          <p>
            Vindue needs <strong>Accessibility</strong> access to move and
            resize windows.
          </p>
          <button
            onClick={() => {
              // Dismiss first: otherwise the target watcher would retarget to
              // (and outline) the System Settings window this opens — the user
              // is granting permission, not choosing a resize target. Re-grant
              // flow: grant → re-activate panel → fresh target, banner gone.
              dismissPanel();
              axTrusted(true).then(setAxOk);
            }}
          >
            Grant access…
          </button>
          {grantTarget && (
            <div className="exe">
              <code title={grantTarget}>{grantTarget}</code>
              <button className="copy" onClick={copyPath}>
                {copied ? "Copied" : "Copy path"}
              </button>
            </div>
          )}
          <p className="hint">
            In System Settings → Accessibility: remove any old entries (−),
            then add this exact file (+ or drag it in) and enable it. Rebuilds
            change the app's identity, so stale entries never apply. Granting
            dismisses the panel — re-activate it afterwards; it detects the
            grant automatically.
          </p>
        </div>
      )}
    </div>
  );
}

export default App;
