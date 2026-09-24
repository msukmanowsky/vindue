// Pure geometry: grid selection -> screen rect.
// All values are PHYSICAL PIXELS in global screen space (Tauri monitor units).
// The Rust side divides by the monitor's scaleFactor to get macOS points.

/**
 * The `grid` config section: subdivision plus spacing. Mirrored 1:1 by the
 * Rust `GridCfg` in src-tauri/src/config.rs — golden-vector fixtures in
 * fixtures/ pin both implementations to the same behavior.
 */
export interface GridConfig {
  rows: number;
  cols: number;
  /** Gap between adjacent placed windows; each window is inset by half. */
  windowGap: { width: number; height: number };
  /** Insets from the screen (work area) edges. */
  screenMargins: { top: number; right: number; bottom: number; left: number };
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Cell {
  row: number;
  col: number;
}

/** Inclusive normalized selection. */
export interface Selection {
  startRow: number;
  endRow: number;
  startCol: number;
  endCol: number;
}

export function selectionFromCells(a: Cell, b: Cell): Selection {
  return {
    startRow: Math.min(a.row, b.row),
    endRow: Math.max(a.row, b.row),
    startCol: Math.min(a.col, b.col),
    endCol: Math.max(a.col, b.col),
  };
}

/**
 * Best-effort remap of a selection onto a resized grid: edges scale
 * proportionally (exclusive end edge), so "left half", "full", "top-left
 * quadrant" survive a rows/cols change. Selections degenerate to at least
 * one cell when rounding would erase them.
 */
export function rescaleSelection(
  sel: Selection,
  from: { rows: number; cols: number },
  to: { rows: number; cols: number },
): Selection {
  const span = (start: number, end: number, oldN: number, newN: number) => {
    const s = Math.min(Math.round((start * newN) / oldN), newN - 1);
    const e = Math.max(Math.round(((end + 1) * newN) / oldN) - 1, s);
    return [s, e] as const;
  };
  const [startRow, endRow] = span(sel.startRow, sel.endRow, from.rows, to.rows);
  const [startCol, endCol] = span(sel.startCol, sel.endCol, from.cols, to.cols);
  return { startRow, endRow, startCol, endCol };
}

export function selectionToRect(sel: Selection, grid: GridConfig, area: Rect): Rect {
  const inner: Rect = {
    x: area.x + grid.screenMargins.left,
    y: area.y + grid.screenMargins.top,
    width: Math.max(0, area.width - grid.screenMargins.left - grid.screenMargins.right),
    height: Math.max(0, area.height - grid.screenMargins.top - grid.screenMargins.bottom),
  };

  const cellWidth = inner.width / grid.cols;
  const cellHeight = inner.height / grid.rows;

  const raw: Rect = {
    x: inner.x + sel.startCol * cellWidth,
    y: inner.y + sel.startRow * cellHeight,
    width: (sel.endCol - sel.startCol + 1) * cellWidth,
    height: (sel.endRow - sel.startRow + 1) * cellHeight,
  };

  const insetX = grid.windowGap.width / 2;
  const insetY = grid.windowGap.height / 2;

  return {
    x: raw.x + insetX,
    y: raw.y + insetY,
    width: Math.max(0, raw.width - 2 * insetX),
    height: Math.max(0, raw.height - 2 * insetY),
  };
}

/**
 * Named presets for API/MCP tiling, computed proportionally from the grid so
 * they work at any rows/cols. Unknown preset → null. Shared vocabulary with
 * the Rust `preset_selection` port (fixtures/preset-cases.json pins both).
 */
export function presetSelection(name: string, grid: { rows: number; cols: number }): Selection | null {
  const half = (n: number) => Math.max(1, Math.round(n / 2));
  const rows = grid.rows;
  const cols = grid.cols;
  const hRows = half(rows);
  const hCols = half(cols);
  switch (name) {
    case "full":
      return { startRow: 0, endRow: rows - 1, startCol: 0, endCol: cols - 1 };
    case "left_half":
      return { startRow: 0, endRow: rows - 1, startCol: 0, endCol: hCols - 1 };
    case "right_half":
      return { startRow: 0, endRow: rows - 1, startCol: cols - hCols, endCol: cols - 1 };
    case "top_half":
      return { startRow: 0, endRow: hRows - 1, startCol: 0, endCol: cols - 1 };
    case "bottom_half":
      return { startRow: rows - hRows, endRow: rows - 1, startCol: 0, endCol: cols - 1 };
    case "top_left":
      return { startRow: 0, endRow: hRows - 1, startCol: 0, endCol: hCols - 1 };
    case "top_right":
      return { startRow: 0, endRow: hRows - 1, startCol: cols - hCols, endCol: cols - 1 };
    case "bottom_left":
      return { startRow: rows - hRows, endRow: rows - 1, startCol: 0, endCol: hCols - 1 };
    case "bottom_right":
      return { startRow: rows - hRows, endRow: rows - 1, startCol: cols - hCols, endCol: cols - 1 };
    default:
      return null;
  }
}
