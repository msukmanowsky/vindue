import { describe, expect, it } from "vitest";
import {
  presetSelection,
  rescaleSelection,
  selectionFromCells,
  selectionToRect,
  type GridConfig,
} from "./geometry";

const grid: GridConfig = {
  rows: 6,
  cols: 6,
  windowGap: { width: 10, height: 10 },
  screenMargins: { top: 10, right: 10, bottom: 10, left: 10 },
};
const screen = { x: 0, y: 0, width: 2560, height: 1600 };

describe("selectionFromCells", () => {
  it("normalizes a reversed drag", () => {
    expect(selectionFromCells({ row: 4, col: 5 }, { row: 1, col: 2 })).toEqual({
      startRow: 1,
      endRow: 4,
      startCol: 2,
      endCol: 5,
    });
  });
});

describe("rescaleSelection", () => {
  const g6 = { rows: 6, cols: 6 };

  it("preserves left-half intent from 6x6 to 8x8", () => {
    expect(
      rescaleSelection({ startRow: 0, endRow: 5, startCol: 0, endCol: 2 }, g6, { rows: 8, cols: 8 }),
    ).toEqual({ startRow: 0, endRow: 7, startCol: 0, endCol: 3 });
  });

  it("full grid stays full", () => {
    expect(
      rescaleSelection({ startRow: 0, endRow: 5, startCol: 0, endCol: 5 }, g6, { rows: 1, cols: 12 }),
    ).toEqual({ startRow: 0, endRow: 0, startCol: 0, endCol: 11 });
  });

  it("top-left quadrant survives an odd resize", () => {
    expect(
      rescaleSelection({ startRow: 0, endRow: 2, startCol: 0, endCol: 2 }, g6, { rows: 5, cols: 7 }),
    ).toEqual({ startRow: 0, endRow: 2, startCol: 0, endCol: 3 });
  });

  it("shrinking keeps proportions (8x8 right half -> 3x3)", () => {
    expect(
      rescaleSelection(
        { startRow: 0, endRow: 7, startCol: 4, endCol: 7 },
        { rows: 8, cols: 8 },
        { rows: 3, cols: 3 },
      ),
    ).toEqual({ startRow: 0, endRow: 2, startCol: 2, endCol: 2 });
  });

  it("a single cell degenerates to one cell, never zero-width", () => {
    const s = rescaleSelection({ startRow: 5, endRow: 5, startCol: 5, endCol: 5 }, g6, { rows: 1, cols: 1 });
    expect(s).toEqual({ startRow: 0, endRow: 0, startCol: 0, endCol: 0 });
  });

  it("same grid is identity", () => {
    const sel = { startRow: 2, endRow: 3, startCol: 1, endCol: 4 };
    expect(rescaleSelection(sel, g6, g6)).toEqual(sel);
  });
});

describe("selectionToRect", () => {
  it("full grid = work area minus screen margins and half window gap", () => {
    const rect = selectionToRect(
      { startRow: 0, endRow: 5, startCol: 0, endCol: 5 },
      grid,
      screen,
    );
    expect(rect).toEqual({ x: 15, y: 15, width: 2530, height: 1570 });
  });

  it("single top-left cell", () => {
    const rect = selectionToRect(
      { startRow: 0, endRow: 0, startCol: 0, endCol: 0 },
      grid,
      screen,
    );
    const cellWidth = (2560 - 20) / 6;
    const cellHeight = (1600 - 20) / 6;
    expect(rect.x).toBeCloseTo(15);
    expect(rect.y).toBeCloseTo(15);
    expect(rect.width).toBeCloseTo(cellWidth - 10);
    expect(rect.height).toBeCloseTo(cellHeight - 10);
  });

  it("zero margins = exact proportional split", () => {
    const zero: GridConfig = {
      rows: 6,
      cols: 6,
      windowGap: { width: 0, height: 0 },
      screenMargins: { top: 0, right: 0, bottom: 0, left: 0 },
    };
    const rightHalf = selectionToRect(
      { startRow: 0, endRow: 5, startCol: 3, endCol: 5 },
      zero,
      screen,
    );
    expect(rightHalf).toEqual({ x: 1280, y: 0, width: 1280, height: 1600 });
  });

  it("respects monitor origin (secondary display above-right)", () => {
    const secondary = { x: 2560, y: -500, width: 1920, height: 1080 };
    const rect = selectionToRect(
      { startRow: 0, endRow: 5, startCol: 0, endCol: 5 },
      grid,
      secondary,
    );
    expect(rect.x).toBe(2575);
    expect(rect.y).toBe(-485);
    expect(rect.width).toBe(1890);
    expect(rect.height).toBe(1050);
  });

  it("adjacent selections leave exactly the window gap between them", () => {
    const left = selectionToRect(
      { startRow: 0, endRow: 5, startCol: 0, endCol: 2 },
      grid,
      screen,
    );
    const right = selectionToRect(
      { startRow: 0, endRow: 5, startCol: 3, endCol: 5 },
      grid,
      screen,
    );
    expect(right.x - (left.x + left.width)).toBeCloseTo(10);
  });

  it("screen margins larger than the area clamp to zero size", () => {
    const huge: GridConfig = {
      rows: 6,
      cols: 6,
      windowGap: { width: 0, height: 0 },
      screenMargins: { top: 900, right: 0, bottom: 900, left: 0 },
    };
    const rect = selectionToRect(
      { startRow: 0, endRow: 5, startCol: 0, endCol: 5 },
      huge,
      screen,
    );
    expect(rect.height).toBe(0);
  });
});

describe("presetSelection", () => {
  it("left_half on an odd grid rounds the half up", () => {
    expect(presetSelection("left_half", { rows: 7, cols: 7 })).toEqual({
      startRow: 0,
      endRow: 6,
      startCol: 0,
      endCol: 3,
    });
  });

  it("bottom_right mirrors the rounding", () => {
    expect(presetSelection("bottom_right", { rows: 7, cols: 7 })).toEqual({
      startRow: 3,
      endRow: 6,
      startCol: 3,
      endCol: 6,
    });
  });

  it("unknown presets return null", () => {
    expect(presetSelection("bogus", { rows: 6, cols: 6 })).toBeNull();
  });
});
