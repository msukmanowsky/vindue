// Typed wrappers around the app's Rust commands.
import { invoke } from "@tauri-apps/api/core";
import type { Rect } from "./geometry";

export interface Target {
  pid: number;
  app_name: string;
  /** CGWindowNumber — distinguishes windows within the same app. */
  wid: number;
  /** Window bounds in points (global screen space), from CGWindowList. */
  x: number;
  y: number;
  w: number;
  h: number;
}

export function axTrusted(prompt: boolean): Promise<boolean> {
  return invoke<boolean>("ax_trusted", { prompt });
}

export function currentTarget(): Promise<Target | null> {
  return invoke<Target | null>("current_target");
}

export function currentExe(): Promise<string | null> {
  return invoke<string | null>("current_exe");
}

export function applyRect(rect: Rect, scale: number): Promise<void> {
  return invoke<void>("apply_rect", {
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
    scale,
  });
}

/** Hide panel + target highlight strips. */
export function dismissPanel(): Promise<void> {
  return invoke<void>("dismiss");
}

export function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

/**
 * The one save pipeline: validate (Rust twin of the Yup schema) → store
 * write → broadcast `config-saved` to open panels → swap the global hotkey
 * if changed → rebind the API server if changed. Rejects with a joined
 * validation-error or hotkey-conflict message.
 */
export function commitConfig(configValue: Record<string, unknown>): Promise<void> {
  return invoke<void>("commit_config", { configValue });
}

/**
 * Temporarily enable/disable the global hotkey — the recorder disables it
 * so pressing the currently registered combination reaches the webview
 * instead of firing (the OS intercepts global shortcuts before any app).
 */
export function setHotkeyEnabled(hotkey: string, enabled: boolean): Promise<void> {
  return invoke<void>("set_hotkey_enabled", { hotkey, enabled });
}

/** Fingerprint (mtime+size) of config.json on disk; null when missing. */
export function configStamp(): Promise<string | null> {
  return invoke<string | null>("config_stamp");
}

/** Re-read config.json from disk into the store plugin's in-memory cache. */
export function reloadConfig(): Promise<void> {
  return invoke<void>("reload_config");
}

/** Human-friendly display names, keyed by CGDisplayModelNumber (the number
 *  tao embeds in its "Monitor #<n>" placeholder names). */
export function monitorNames(): Promise<[number, string][]> {
  return invoke<[number, string][]>("monitor_names");
}

/** 64×64 PNG icon of the app with this pid (raw bytes; empty if none). */
export function appIcon(pid: number): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("app_icon", { pid });
}

/** Running regular apps for the app-picker: (pid, localized name). */
export function runningApps(): Promise<[number, string][]> {
  return invoke<[number, string][]>("running_apps");
}

/** Retarget the panels at a specific app's frontmost window. */
export function targetApp(pid: number): Promise<Target | null> {
  return invoke<Target | null>("target_app", { pid });
}

/** Live status of the loopback HTTP/MCP server. */
export interface ApiInfo {
  enabled: boolean;
  configuredPort: number | null;
  boundPort: number | null;
  running: boolean;
}

export function apiInfo(): Promise<ApiInfo> {
  return invoke<ApiInfo>("api_info");
}
