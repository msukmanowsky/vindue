import { describe, expect, it } from "vitest";
import {
  displayIdFromName,
  friendlyMonitorName,
  monitorLabels,
  resolveMonitorIndex,
  shortcutColor,
  SHORTCUT_COLORS,
  SHORTCUT_KEYS,
  SHORTCUT_KEY_RE,
  type MonitorLike,
} from "./shortcuts";

const monitors: MonitorLike[] = [
  { name: "Built-in Retina Display", position: { x: 0, y: 0 } },
  { name: "DELL U2720Q", position: { x: 1920, y: 0 } },
  { name: null, position: { x: -1000, y: 0 } },
];

describe("resolveMonitorIndex", () => {
  it("resolves null (relative) to the fallback without flagging", () => {
    expect(resolveMonitorIndex(null, monitors, 1)).toEqual({
      index: 1,
      fellBack: false,
    });
  });

  it("prefers an exact name match over the saved index", () => {
    // DELL was saved at index 1 but is now enumerated second => still exact.
    expect(
      resolveMonitorIndex({ name: "DELL U2720Q", index: 0 }, monitors, 0),
    ).toEqual({ index: 1, fellBack: false });
  });

  it("falls back to the saved index when the name disappeared", () => {
    expect(
      resolveMonitorIndex({ name: "Old Display", index: 2 }, monitors, 0),
    ).toEqual({ index: 2, fellBack: true });
  });

  it("falls back to current when name and index are both gone", () => {
    expect(
      resolveMonitorIndex({ name: "Old Display", index: 9 }, monitors, 1),
    ).toEqual({ index: 1, fellBack: true });
  });

  it("resolves a nameless ref by index without flagging", () => {
    expect(resolveMonitorIndex({ name: null, index: 2 }, monitors, 0)).toEqual({
      index: 2,
      fellBack: false,
    });
  });

  it("falls back to current for a nameless ref with an out-of-range index", () => {
    expect(resolveMonitorIndex({ name: null, index: 5 }, monitors, 0)).toEqual({
      index: 0,
      fellBack: true,
    });
  });
});

describe("shortcutColor", () => {
  it("is deterministic for the same key set", () => {
    const codes = ["Digit1", "Backquote", "Digit3"];
    expect(shortcutColor("Digit1", codes)).toBe(shortcutColor("Digit1", [...codes].reverse()));
  });

  it("assigns unique colors within the palette size", () => {
    const codes = Object.keys(SHORTCUT_KEYS).slice(0, SHORTCUT_COLORS.length);
    const colors = new Set(codes.map((c) => shortcutColor(c, codes)));
    expect(colors.size).toBe(codes.length);
  });
});

describe("SHORTCUT_KEY_RE", () => {
  it("matches digits and punctuation codes", () => {
    for (const code of ["Digit0", "Digit9", "Backquote", "Minus", "Slash"]) {
      expect(SHORTCUT_KEY_RE.test(code)).toBe(true);
    }
  });

  it("rejects legacy digits, letters, and modified codes", () => {
    for (const code of ["1", "KeyQ", "Digit10", "ShiftLeft", "Space"]) {
      expect(SHORTCUT_KEY_RE.test(code)).toBe(false);
    }
  });

  it("covers every label key", () => {
    for (const code of Object.keys(SHORTCUT_KEYS)) {
      expect(SHORTCUT_KEY_RE.test(code)).toBe(true);
    }
  });
});

describe("displayIdFromName", () => {
  it("extracts the CGDirectDisplayID from tao's placeholder", () => {
    expect(displayIdFromName("Monitor #41042")).toBe(41042);
  });

  it("returns null for non-placeholder names and null", () => {
    expect(displayIdFromName("DELL U2720Q")).toBe(null);
    expect(displayIdFromName("Monitor #")).toBe(null);
    expect(displayIdFromName(null)).toBe(null);
  });
});

describe("friendlyMonitorName", () => {
  const names = new Map([
    [41042, "DELL U2720Q"],
    [1, "Built-in Retina Display"],
  ]);

  it("maps a tao placeholder to the localized name", () => {
    expect(friendlyMonitorName("Monitor #41042", names)).toBe("DELL U2720Q");
    expect(friendlyMonitorName("Monitor #1", names)).toBe("Built-in Retina Display");
  });

  it("passes through an unknown placeholder unchanged", () => {
    expect(friendlyMonitorName("Monitor #999", names)).toBe("Monitor #999");
  });

  it("passes through already-friendly names and null", () => {
    expect(friendlyMonitorName("DELL U2720Q", names)).toBe("DELL U2720Q");
    expect(friendlyMonitorName(null, names)).toBe(null);
  });
});

describe("monitorLabels", () => {
  const names = new Map([
    [100, "DELL U2720Q"],
    [200, "Built-in Retina Display"],
  ]);

  it("leaves unique friendly names unsuffixed", () => {
    const ms: MonitorLike[] = [
      { name: "Monitor #200", position: { x: 0, y: 0 } },
      { name: "Monitor #100", position: { x: 1920, y: 0 } },
    ];
    expect(monitorLabels(ms, names)).toEqual([
      "Built-in Retina Display",
      "DELL U2720Q",
    ]);
  });

  it("suffixes identical twins left/right by x order", () => {
    const ms: MonitorLike[] = [
      { name: "Monitor #100", position: { x: 2560, y: 0 } },
      { name: "Monitor #100", position: { x: 0, y: 0 } },
    ];
    expect(monitorLabels(ms, names)).toEqual([
      "DELL U2720Q (right)",
      "DELL U2720Q (left)",
    ]);
  });

  it("labels triplets left/middle/right with a vertical tiebreak", () => {
    const ms: MonitorLike[] = [
      { name: "Monitor #100", position: { x: 0, y: 1000 } },
      { name: "Monitor #100", position: { x: 0, y: 0 } },
      { name: "Monitor #100", position: { x: 2560, y: 0 } },
    ];
    expect(monitorLabels(ms, names)).toEqual([
      "DELL U2720Q (middle)",
      "DELL U2720Q (left)",
      "DELL U2720Q (right)",
    ]);
  });

  it("numbers four or more left-to-right", () => {
    const ms: MonitorLike[] = [3000, 1000, 2000, 0].map((x) => ({
      name: "Monitor #100",
      position: { x, y: 0 },
    }));
    expect(monitorLabels(ms, names)).toEqual([
      "DELL U2720Q (left-to-right #4)",
      "DELL U2720Q (left-to-right #2)",
      "DELL U2720Q (left-to-right #3)",
      "DELL U2720Q (left-to-right #1)",
    ]);
  });

  it("numbers unnamed displays distinctly, no suffixes", () => {
    const ms: MonitorLike[] = [
      { name: null, position: { x: 0, y: 0 } },
      { name: null, position: { x: 1000, y: 0 } },
    ];
    expect(monitorLabels(ms, new Map())).toEqual(["Display 1", "Display 2"]);
  });
});

describe("resolveMonitorIndex with labels", () => {
  const names = new Map([[100, "DELL U2720Q"]]);
  const twins: MonitorLike[] = [
    { name: "Monitor #100", position: { x: 0, y: 0 } },
    { name: "Monitor #100", position: { x: 2560, y: 0 } },
  ];

  it("matches the disambiguated label", () => {
    expect(
      resolveMonitorIndex({ name: "DELL U2720Q (right)", index: 0 }, twins, 0, names),
    ).toEqual({ index: 1, fellBack: false });
  });

  it("falls back to the base name when the twin is unplugged", () => {
    const one: MonitorLike[] = [{ name: "Monitor #100", position: { x: 0, y: 0 } }];
    expect(
      resolveMonitorIndex({ name: "DELL U2720Q (right)", index: 5 }, one, 0, names),
    ).toEqual({ index: 0, fellBack: false });
  });

  it("resolves a friendly name against raw tao names (name-first works)", () => {
    const one: MonitorLike[] = [{ name: "Monitor #100", position: { x: 0, y: 0 } }];
    expect(
      resolveMonitorIndex({ name: "DELL U2720Q", index: 9 }, one, 0, names),
    ).toEqual({ index: 0, fellBack: false });
  });
});
