import { describe, expect, it } from "vitest";
import { validateConfig } from "./configSchema";

const sel = { startRow: 0, endRow: 5, startCol: 0, endCol: 2 };

const valid = {
  version: 1,
  grid: {
    rows: 6,
    cols: 6,
    windowGap: { width: 10, height: 10 },
    screenMargins: { top: 10, right: 10, bottom: 10, left: 10 },
  },
  keybindings: { openPanel: "Cmd+Shift+Space" },
  shortcuts: {
    assignment: "pinned",
    keys: {
      Digit1: { monitor: null, selection: sel },
      Backquote: {
        monitor: { name: "DELL U2720Q", index: 1 },
        selection: { startRow: 0, endRow: 5, startCol: 0, endCol: 5 },
      },
    },
  },
  api: { port: 47725, enabled: true },
};

/** `valid` with shortcuts.keys replaced. */
const withKeys = (keys: Record<string, unknown>) => ({
  ...valid,
  shortcuts: { ...valid.shortcuts, keys },
});

describe("configSchema", () => {
  it("accepts a valid config", async () => {
    await expect(validateConfig(valid)).resolves.toBeDefined();
  });

  it("accepts empty shortcut keys", async () => {
    await expect(validateConfig(withKeys({}))).resolves.toBeDefined();
  });

  it("accepts an empty open-panel hotkey", async () => {
    await expect(
      validateConfig({ ...valid, keybindings: { openPanel: "" } }),
    ).resolves.toBeDefined();
  });

  it("accepts relative assignment", async () => {
    await expect(
      validateConfig({
        ...valid,
        shortcuts: { ...valid.shortcuts, assignment: "relative" },
      }),
    ).resolves.toBeDefined();
  });

  it("accepts nameless monitor refs", async () => {
    await expect(
      validateConfig(
        withKeys({ Digit2: { monitor: { name: null, index: 0 }, selection: sel } }),
      ),
    ).resolves.toBeDefined();
  });

  it("rejects legacy single-digit keys", async () => {
    await expect(
      validateConfig(withKeys({ "1": { monitor: null, selection: sel } })),
    ).rejects.toThrow();
  });

  it("rejects unknown key codes", async () => {
    await expect(
      validateConfig(withKeys({ KeyQ: { monitor: null, selection: sel } })),
    ).rejects.toThrow();
  });

  it("rejects legacy bare-selection values", async () => {
    await expect(validateConfig(withKeys({ Digit1: sel }))).rejects.toThrow();
  });

  it("rejects unknown monitor-ref keys", async () => {
    await expect(
      validateConfig(
        withKeys({
          Digit1: { monitor: { name: "X", index: 0, id: "abc" }, selection: sel },
        }),
      ),
    ).rejects.toThrow();
  });

  it("rejects selections outside the grid", async () => {
    await expect(
      validateConfig(
        withKeys({
          Digit1: {
            monitor: null,
            selection: { startRow: 0, endRow: 7, startCol: 0, endCol: 2 },
          },
        }),
      ),
    ).rejects.toThrow();
  });

  it("rejects reversed selections", async () => {
    await expect(
      validateConfig(
        withKeys({
          Digit1: {
            monitor: null,
            selection: { startRow: 5, endRow: 0, startCol: 0, endCol: 0 },
          },
        }),
      ),
    ).rejects.toThrow();
  });

  it("rejects window gaps above 300", async () => {
    await expect(
      validateConfig({
        ...valid,
        grid: { ...valid.grid, windowGap: { width: 500, height: 10 } },
      }),
    ).rejects.toThrow();
  });

  it("rejects negative screen margins", async () => {
    await expect(
      validateConfig({
        ...valid,
        grid: {
          ...valid.grid,
          screenMargins: { top: -1, right: 0, bottom: 0, left: 0 },
        },
      }),
    ).rejects.toThrow();
  });

  it("rejects unknown top-level keys", async () => {
    await expect(validateConfig({ ...valid, bogus: 1 })).rejects.toThrow();
  });

  it("rejects legacy flat keys", async () => {
    await expect(validateConfig({ ...valid, hotkey: "" })).rejects.toThrow();
    await expect(validateConfig({ ...valid, panelAssignment: "pinned" })).rejects.toThrow();
  });

  it("rejects bogus assignment", async () => {
    await expect(
      validateConfig({
        ...valid,
        shortcuts: { ...valid.shortcuts, assignment: "bogus" },
      }),
    ).rejects.toThrow();
  });

  it("rejects grid rows above 12, accepts 12", async () => {
    await expect(
      validateConfig({ ...valid, grid: { ...valid.grid, rows: 13 } }),
    ).rejects.toThrow();
    await expect(
      validateConfig({ ...valid, grid: { ...valid.grid, rows: 12, cols: 12 } }),
    ).resolves.toBeDefined();
  });

  it("enforces api port bounds", async () => {
    await expect(
      validateConfig({ ...valid, api: { port: 1023, enabled: true } }),
    ).rejects.toThrow();
    await expect(
      validateConfig({ ...valid, api: { port: 65536, enabled: true } }),
    ).rejects.toThrow();
    await expect(
      validateConfig({ ...valid, api: { port: 1024, enabled: true } }),
    ).resolves.toBeDefined();
    await expect(
      validateConfig({ ...valid, api: { port: 65535, enabled: true } }),
    ).resolves.toBeDefined();
  });

  it("rejects missing sections", async () => {
    const { api: _api, ...noApi } = valid;
    await expect(validateConfig(noApi)).rejects.toThrow();
    const { keybindings: _kb, ...noKb } = valid;
    await expect(validateConfig(noKb)).rejects.toThrow();
  });
});
