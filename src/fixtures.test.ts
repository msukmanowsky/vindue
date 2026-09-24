// Golden-vector fixture tests: every case in fixtures/*.json is run through
// the TS implementations here AND the Rust ports in src-tauri/src/config.rs
// (see the fixture tests at the bottom of that file). The fixtures are the
// contract — changing behavior on one side without the other fails one of
// the two suites.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  presetSelection,
  rescaleSelection,
  selectionToRect,
  type GridConfig,
  type Rect,
  type Selection,
} from "./geometry";
import {
  monitorLabels,
  resolveMonitorIndex,
  type MonitorLike,
  type MonitorRef,
} from "./shortcuts";
import { validateConfig } from "./configSchema";

function read(file: string): { cases: Record<string, unknown>[] } {
  return JSON.parse(readFileSync(new URL(`../fixtures/${file}`, import.meta.url), "utf8"));
}

const asSel = (v: unknown): Selection => {
  const o = v as Record<string, number>;
  return {
    startRow: o.startRow,
    endRow: o.endRow,
    startCol: o.startCol,
    endCol: o.endCol,
  };
};

const asRect = (v: unknown): Rect => {
  const o = v as Record<string, number>;
  return { x: o.x, y: o.y, width: o.width, height: o.height };
};

interface MonCase {
  name: string | null;
  x: number;
  y: number;
}

const asMonitors = (v: unknown): MonitorLike[] =>
  (v as MonCase[]).map((m) => ({ name: m.name, position: { x: m.x, y: m.y } }));

const asNameMap = (v: unknown): Map<number, string> =>
  new Map(Object.entries(v as Record<string, string>).map(([k, n]) => [Number(k), n]));

describe("fixtures/validate-cases.json", () => {
  const { cases } = read("validate-cases.json");
  it.each(cases.map((c) => [c.name as string, c] as const))(
    "%s",
    async (_name, c) => {
      if (c.valid) {
        await expect(validateConfig(c.config)).resolves.toBeDefined();
      } else {
        await expect(validateConfig(c.config)).rejects.toThrow();
      }
    },
  );
});

describe("fixtures/rescale-cases.json", () => {
  const { cases } = read("rescale-cases.json");
  it.each(cases.map((c) => [c.name as string, c] as const))("%s", (_name, c) => {
    expect(
      rescaleSelection(
        asSel(c.sel),
        c.from as { rows: number; cols: number },
        c.to as { rows: number; cols: number },
      ),
    ).toEqual(asSel(c.expect));
  });
});

describe("fixtures/rect-cases.json", () => {
  const { cases } = read("rect-cases.json");
  it.each(cases.map((c) => [c.name as string, c] as const))("%s", (_name, c) => {
    const rect = selectionToRect(asSel(c.sel), c.grid as GridConfig, asRect(c.area));
    const expectRect = asRect(c.expect);
    expect(rect.x).toBeCloseTo(expectRect.x, 6);
    expect(rect.y).toBeCloseTo(expectRect.y, 6);
    expect(rect.width).toBeCloseTo(expectRect.width, 6);
    expect(rect.height).toBeCloseTo(expectRect.height, 6);
  });
});

describe("fixtures/preset-cases.json", () => {
  const { cases } = read("preset-cases.json");
  it.each(cases.map((c) => [c.name as string, c] as const))("%s", (_name, c) => {
    const got = presetSelection(c.preset as string, c.grid as { rows: number; cols: number });
    expect(got).toEqual(c.expect == null ? null : asSel(c.expect));
  });
});

describe("fixtures/label-cases.json", () => {
  const { cases } = read("label-cases.json");
  it.each(cases.map((c) => [c.name as string, c] as const))("%s", (_name, c) => {
    expect(monitorLabels(asMonitors(c.monitors), asNameMap(c.names))).toEqual(c.expect);
  });
});

describe("fixtures/resolve-cases.json", () => {
  const { cases } = read("resolve-cases.json");
  it.each(cases.map((c) => [c.name as string, c] as const))("%s", (_name, c) => {
    const r = c.ref as (MonitorRef & { name?: string | null }) | null;
    const ref: MonitorRef | null =
      r == null ? null : { name: r.name ?? null, index: r.index };
    const got = resolveMonitorIndex(
      ref,
      asMonitors(c.monitors),
      c.fallback as number,
      asNameMap(c.names),
    );
    expect(got).toEqual(c.expect);
  });
});
