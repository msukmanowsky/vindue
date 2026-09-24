// Standardized schema for config.json, enforced with Yup in the Settings UI.
// Top-level keys mirror the Settings UI sections: grid, keybindings,
// shortcuts, api. The Rust port (validate_config in src-tauri/src/config.rs)
// is pinned to the same accept/reject behavior by fixtures/validate-cases.json
// — both suites consume it.
import * as yup from "yup";
import { SHORTCUT_KEY_RE } from "./shortcuts";

const selectionSchema = yup
  .object({
    startRow: yup.number().integer().min(0).required(),
    endRow: yup.number().integer().min(0).required(),
    startCol: yup.number().integer().min(0).required(),
    endCol: yup.number().integer().min(0).required(),
  })
  .noUnknown()
  .test(
    "ordered",
    "selection start must be <= end",
    (v) => v == null || (v.startRow <= v.endRow && v.startCol <= v.endCol),
  );

const monitorRefSchema = yup
  .object({
    name: yup.string().nullable().defined(),
    index: yup.number().integer().min(0).required(),
  })
  .noUnknown()
  .required();

const shortcutDefSchema = yup
  .object({
    monitor: monitorRefSchema.nullable().defined(),
    selection: selectionSchema.required(),
  })
  .noUnknown()
  .required();

const pxSchema = (name: string) =>
  yup.number().integer().min(0).max(300).required(`${name} must be 0-300`);

export const configSchema = yup
  .object({
    version: yup.number().integer().required(),
    grid: yup
      .object({
        rows: yup.number().integer().min(1).max(12).required(),
        cols: yup.number().integer().min(1).max(12).required(),
        windowGap: yup
          .object({
            width: pxSchema("grid.windowGap.width"),
            height: pxSchema("grid.windowGap.height"),
          })
          .noUnknown()
          .required(),
        screenMargins: yup
          .object({
            top: pxSchema("grid.screenMargins.top"),
            right: pxSchema("grid.screenMargins.right"),
            bottom: pxSchema("grid.screenMargins.bottom"),
            left: pxSchema("grid.screenMargins.left"),
          })
          .noUnknown()
          .required(),
      })
      .noUnknown()
      .required(),
    keybindings: yup
      .object({
        // "" = no global hotkey (the tray still opens the panel). Note:
        // .required() would reject the empty string — .defined() is the
        // parity match for the Rust check (present and a string).
        openPanel: yup.string().defined(),
      })
      .noUnknown()
      .required(),
    shortcuts: yup
      .object({
        assignment: yup
          .string()
          .oneOf(["pinned", "relative"], "shortcuts.assignment must be \"pinned\" or \"relative\"")
          .required(),
        keys: yup
          .mixed()
          .test(
            "shortcut-keys",
            "shortcuts.keys must map assignable keys (digits, ` - = [ ] \\ ; ' , . /) to { monitor, selection }",
            async (value) => {
              if (value == null) return true;
              if (typeof value !== "object" || Array.isArray(value)) return false;
              for (const [key, def] of Object.entries(value as Record<string, unknown>)) {
                if (!SHORTCUT_KEY_RE.test(key)) return false;
                try {
                  await shortcutDefSchema.validate(def, { strict: true });
                } catch {
                  return false;
                }
              }
              return true;
            },
          )
          .required(),
      })
      .noUnknown()
      .required(),
    api: yup
      .object({
        port: yup.number().integer().min(1024).max(65535).required(),
        enabled: yup.boolean().required(),
      })
      .noUnknown()
      .required(),
  })
  .noUnknown()
  .test(
    "bounds",
    "shortcut selections must fit within grid rows/cols",
    (v) => {
      if (v == null) return true;
      const grid = v.grid as { rows?: number; cols?: number } | undefined;
      const shortcuts = v.shortcuts as
        | { keys?: Record<string, { selection?: { endRow?: number; endCol?: number } }> }
        | undefined;
      if (!grid || typeof grid.rows !== "number" || typeof grid.cols !== "number") return true;
      for (const def of Object.values(shortcuts?.keys ?? {})) {
        const s = def?.selection;
        if (!s) continue;
        if (
          typeof s.endRow === "number" &&
          typeof s.endCol === "number" &&
          (s.endRow >= grid.rows || s.endCol >= grid.cols)
        ) {
          return false;
        }
      }
      return true;
    },
  )
  .required();

/**
 * Validate a parsed config.json object. strict: no cast — JSON.parse already
 * gives correct types, and yup's default cast would silently strip unknown
 * keys instead of rejecting them.
 */
export async function validateConfig(value: unknown): Promise<Record<string, unknown>> {
  return (await configSchema.validate(value, {
    abortEarly: false,
    strict: true,
  })) as Record<string, unknown>;
}
