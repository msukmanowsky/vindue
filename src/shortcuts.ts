// Shortcut model shared by the panel, the settings UI, and the schema.
import type { Selection } from "./geometry";

/**
 * A monitor as referenced by a saved shortcut. Monitors have no stable OS
 * identity, so we save both the name (preferred, stable for fixed setups)
 * and the enumeration index (fallback). `null` means "relative": apply on
 * whichever display the panel is currently on.
 */
export interface MonitorRef {
  name: string | null;
  index: number;
}

export interface ShortcutDef {
  monitor: MonitorRef | null;
  selection: Selection;
}

/** Assignable keys, keyed by physical key code (layout-independent). */
export const SHORTCUT_KEYS: Record<string, string> = {
  Backquote: "`",
  Digit1: "1",
  Digit2: "2",
  Digit3: "3",
  Digit4: "4",
  Digit5: "5",
  Digit6: "6",
  Digit7: "7",
  Digit8: "8",
  Digit9: "9",
  Digit0: "0",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
};

export const SHORTCUT_KEY_RE =
  /^(Digit[0-9]|Backquote|Minus|Equal|BracketLeft|BracketRight|Backslash|Semicolon|Quote|Comma|Period|Slash)$/;

/** High-contrast colors, distinguishable on the dark panel background. */
export const SHORTCUT_COLORS = [
  "#ff453a",
  "#ff9f0a",
  "#ffd60a",
  "#30d158",
  "#66d4cf",
  "#64d2ff",
  "#0a84ff",
  "#bf5af2",
  "#ff375f",
  "#a3e635",
  "#f472b6",
  "#d2b48c",
];

/** Deterministic unique color per shortcut key (sorted-order palette index). */
export function shortcutColor(code: string, allCodes: Iterable<string>): string {
  const sorted = [...allCodes].sort();
  const i = sorted.indexOf(code);
  return SHORTCUT_COLORS[(i < 0 ? 0 : i) % SHORTCUT_COLORS.length];
}

export interface MonitorLike {
  name: string | null;
  position: { x: number; y: number };
}

/** tao names macOS monitors "Monitor #<CGDisplayModelNumber>"; extract it. */
export function displayIdFromName(name: string | null): number | null {
  if (name == null) return null;
  const m = /^Monitor #(\d+)$/.exec(name);
  return m ? Number(m[1]) : null;
}

/**
 * Best human name for a monitor: the macOS localized display name (what
 * System Settings shows) when known; otherwise the raw Tauri name.
 */
export function friendlyMonitorName(
  name: string | null,
  names: Map<number, string>,
): string | null {
  const id = displayIdFromName(name);
  if (id != null) {
    const friendly = names.get(id);
    if (friendly) return friendly;
  }
  return name;
}

/** Positional suffix for rank `rank` of `n` identical-name displays. */
function positionWord(rank: number, n: number): string {
  if (n === 2) return rank === 0 ? "left" : "right";
  if (n === 3) return ["left", "middle", "right"][rank];
  return `left-to-right #${rank + 1}`;
}

/** Suffix pattern appended to disambiguate identical-name displays. */
const SUFFIX_RE = / \((?:left|middle|right|left-to-right #\d+)\)$/;

/**
 * The canonical label of every connected monitor, by enumeration index:
 * the friendly name, with a positional suffix ("DELL U2720Q (left)") when
 * several connected displays share the same name. Duplicate groups are
 * ordered by (x, y) in global screen space — macOS persists the display
 * arrangement, so each physical display keeps its position slot across
 * sessions (best effort: rearranging in System Settings re-labels).
 * Labels are recomputed identically at save and apply time; the label,
 * not any hardware identity, is what a saved shortcut stores.
 */
export function monitorLabels(
  monitors: MonitorLike[],
  names: Map<number, string>,
): string[] {
  const base = monitors.map(
    (m, i) => friendlyMonitorName(m.name, names) ?? `Display ${i + 1}`,
  );
  const groups = new Map<string, number[]>();
  base.forEach((b, i) => groups.set(b, [...(groups.get(b) ?? []), i]));
  const labels = [...base];
  for (const [b, idxs] of groups) {
    if (idxs.length < 2) continue;
    const sorted = [...idxs].sort(
      (a, c) =>
        monitors[a].position.x - monitors[c].position.x ||
        monitors[a].position.y - monitors[c].position.y,
    );
    sorted.forEach((mi, rank) => {
      labels[mi] = `${b} (${positionWord(rank, sorted.length)})`;
    });
  }
  return labels;
}

export interface MonitorResolution {
  index: number;
  /** True when the exact saved display could not be honored. */
  fellBack: boolean;
}

/**
 * Resolve a saved monitor reference against the currently connected
 * monitors: label match wins (the canonical label from `monitorLabels`,
 * including positional suffixes for identical displays); then a bare-name
 * match for a suffixed ref whose twin is unplugged; then a raw tao-name
 * match (legacy entries); then index (a soft fallback when a name was
 * saved but no longer matches); then the fallback index. `names` is the
 * model-number → localized-name map; omitting it degrades to raw-name
 * matching only (legacy behavior).
 */
export function resolveMonitorIndex(
  ref: MonitorRef | null,
  monitors: MonitorLike[],
  fallbackIndex: number,
  names?: Map<number, string>,
): MonitorResolution {
  if (ref == null) return { index: fallbackIndex, fellBack: false };
  if (ref.name != null) {
    const labels = monitorLabels(monitors, names ?? new Map());
    const byLabel = labels.indexOf(ref.name);
    if (byLabel >= 0) return { index: byLabel, fellBack: false };
    // Suffixed ref, twin unplugged: the base name now labels exactly one
    // display (duplicate groups are always suffixed).
    const bare = ref.name.replace(SUFFIX_RE, "");
    if (bare !== ref.name) {
      const byBase = labels.indexOf(bare);
      if (byBase >= 0) return { index: byBase, fellBack: false };
    }
    const byRaw = monitors.findIndex((m) => m.name === ref.name);
    if (byRaw >= 0) return { index: byRaw, fellBack: false };
    if (ref.index >= 0 && ref.index < monitors.length) {
      return { index: ref.index, fellBack: true };
    }
    return { index: fallbackIndex, fellBack: true };
  }
  if (ref.index >= 0 && ref.index < monitors.length) {
    return { index: ref.index, fellBack: false };
  }
  return { index: fallbackIndex, fellBack: true };
}
