import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DEFAULT_CONFIG } from "./store";

// Drift guard between the Rust seed (`default_config_json` in
// src-tauri/src/config.rs) and TS DEFAULT_CONFIG: changing either side
// without the other fails these. The Rust side pins the same values in its
// `default_config_is_the_documented_shape` unit test.
const rust = readFileSync(
  new URL("../src-tauri/src/config.rs", import.meta.url),
  "utf8",
);

describe("Rust seed parity", () => {
  it("seeds the same grid", () => {
    const { rows, cols, windowGap, screenMargins } = DEFAULT_CONFIG.grid;
    expect(rust).toContain(`"rows": ${rows}, "cols": ${cols},`);
    expect(rust).toContain(
      `"windowGap": { "width": ${windowGap.width}, "height": ${windowGap.height} },`,
    );
    expect(rust).toContain(
      `"screenMargins": { "top": ${screenMargins.top}, "right": ${screenMargins.right}, "bottom": ${screenMargins.bottom}, "left": ${screenMargins.left} }`,
    );
  });

  it("seeds the same open-panel hotkey", () => {
    expect(rust).toContain(
      `pub const DEFAULT_HOTKEY: &str = "${DEFAULT_CONFIG.keybindings.openPanel}";`,
    );
    expect(rust).toContain('"keybindings": { "openPanel": DEFAULT_HOTKEY },');
  });

  it("seeds the same shortcuts default", () => {
    expect(Object.keys(DEFAULT_CONFIG.shortcuts.keys)).toHaveLength(0);
    expect(rust).toContain(
      `"shortcuts": { "assignment": "${DEFAULT_CONFIG.shortcuts.assignment}", "keys": {} },`,
    );
  });

  it("seeds the same api default", () => {
    expect(rust).toContain(
      `pub const DEFAULT_API_PORT: u16 = ${DEFAULT_CONFIG.api.port};`,
    );
    expect(rust).toContain(
      `"api": { "port": DEFAULT_API_PORT, "enabled": ${DEFAULT_CONFIG.api.enabled} }`,
    );
  });

  it("seeds schema version 1", () => {
    expect(rust).toContain("pub const CONFIG_VERSION: i64 = 1;");
    expect(rust).toContain('"version": CONFIG_VERSION,');
  });
});
