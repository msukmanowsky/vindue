// Config persistence via the official store plugin. The canonical file lives at:
// ~/Library/Application Support/com.oddinteractive.vindue/config.json
// Edit it in the Settings window (schema-validated) or by hand, then relaunch.
// Top-level keys mirror the Settings UI sections 1:1: grid, keybindings,
// shortcuts, api. The Rust seed (src-tauri/src/config.rs) is pinned to this
// shape from both directions — see src/store.test.ts and the Rust
// `default_config_is_the_documented_shape` test.
import { load } from "@tauri-apps/plugin-store";
import type { GridConfig } from "./geometry";
import type { ShortcutDef } from "./shortcuts";

/** What a panel-mode assignment (key + drag) saves as its monitor binding. */
export type PanelAssignment = "pinned" | "relative";

export interface AppConfig {
  grid: GridConfig;
  keybindings: { openPanel: string };
  shortcuts: {
    assignment: PanelAssignment;
    keys: Record<string, ShortcutDef>;
  };
  api: { port: number; enabled: boolean };
}

export const DEFAULT_CONFIG: AppConfig = {
  grid: {
    rows: 6,
    cols: 6,
    windowGap: { width: 0, height: 0 },
    screenMargins: { top: 0, right: 0, bottom: 0, left: 0 },
  },
  keybindings: { openPanel: "" },
  shortcuts: { assignment: "pinned", keys: {} },
  api: { port: 47725, enabled: true },
};

async function openStore() {
  return load("config.json", { autoSave: false });
}

export async function loadConfig(): Promise<AppConfig> {
  try {
    const store = await openStore();
    const [grid, keybindings, shortcuts, api] = await Promise.all([
      store.get<GridConfig>("grid"),
      store.get<AppConfig["keybindings"]>("keybindings"),
      store.get<AppConfig["shortcuts"]>("shortcuts"),
      store.get<AppConfig["api"]>("api"),
    ]);
    return {
      grid: grid ?? DEFAULT_CONFIG.grid,
      keybindings: keybindings ?? DEFAULT_CONFIG.keybindings,
      shortcuts: shortcuts ?? DEFAULT_CONFIG.shortcuts,
      api: api ?? DEFAULT_CONFIG.api,
    };
  } catch {
    return DEFAULT_CONFIG;
  }
}

export async function saveShortcut(key: string, def: ShortcutDef): Promise<void> {
  const store = await openStore();
  const current = (await store.get<AppConfig["shortcuts"]>("shortcuts")) ??
    DEFAULT_CONFIG.shortcuts;
  await store.set("shortcuts", {
    ...current,
    keys: { ...current.keys, [key]: def },
  });
  await store.save();
}

export async function readRawConfig(): Promise<Record<string, unknown>> {
  const store = await openStore();
  return Object.fromEntries(await store.entries());
}
