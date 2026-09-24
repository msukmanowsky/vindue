// Recorder for the global "Open Vindue" hotkey: click to enter recording
// mode, the next valid key combination is captured as a Tauri accelerator
// string. While recording, the currently registered global hotkey is
// temporarily unregistered — the OS intercepts global shortcuts before any
// app sees them, so otherwise pressing the current hotkey would fire the
// panel instead of reaching this recorder. Modifier-only presses are ignored
// while waiting; invalid combinations are rejected with an on-the-spot
// reason (recording stays active); Esc cancels; blur exits; a successful
// capture commits and exits. Token form is the raw DOM key code (e.code) —
// the accelerator parser accepts those directly (verified empirically).
import { useEffect, useRef, useState } from "react";
import { setHotkeyEnabled } from "./bindings";

const MODS: Array<[string, (e: KeyboardEvent) => boolean]> = [
  ["Cmd", (e) => e.metaKey],
  ["Ctrl", (e) => e.ctrlKey],
  ["Alt", (e) => e.altKey],
  ["Shift", (e) => e.shiftKey],
];

const MODIFIER_KEYS = new Set(["Meta", "Control", "Alt", "Shift"]);

/** Combinations macOS reserves for itself (app switching, Spotlight, input sources). */
const RESERVED = new Set(["Cmd+Tab", "Cmd+Shift+Tab", "Cmd+Space", "Ctrl+Space"]);

/** Keys that produce text: letters, digits, and US-layout punctuation. */
const PRINTABLE_RE =
  /^(Key[A-Z]|Digit[0-9]|Minus|Equal|BracketLeft|BracketRight|Backslash|Semicolon|Quote|Comma|Period|Slash|Backquote)$/;

/**
 * Opinionated admissibility rules for a global hotkey. Returns null when the
 * combination is acceptable, otherwise a short human-readable reason:
 *  - at least one modifier (bare keys are panel-shortcut territory);
 *  - nothing macOS reserves (Cmd+Tab, Cmd+Space, …);
 *  - Shift alone just types the key (Shift+A is "A");
 *  - Cmd/Alt alone + a text key belongs to apps / text input (Cmd+C, Alt+E
 *    types dead keys) — add a second modifier or use a non-text key.
 */
export function rejectReason(mods: string[], code: string): string | null {
  if (mods.length === 0) return "needs a modifier";
  if (RESERVED.has([...mods, code].join("+"))) return "reserved by macOS";
  const only = (m: string) => mods.length === 1 && mods[0] === m;
  if (only("Shift")) return "Shift alone just types the key";
  if ((only("Cmd") || only("Alt")) && PRINTABLE_RE.test(code)) {
    return `${mods[0]}+key belongs to apps — add another modifier`;
  }
  return null;
}

export default function HotkeyInput({
  value,
  onChange,
  registeredHotkey,
}: {
  value: string;
  onChange: (v: string) => void;
  /** The hotkey currently registered with the OS (last saved), to pause/resume. */
  registeredHotkey?: string;
}) {
  const [recording, setRecording] = useState(false);
  const [rejected, setRejected] = useState<string | null>(null);
  const ref = useRef<HTMLButtonElement>(null);

  // Pause the live global hotkey while recording; resume on any exit.
  useEffect(() => {
    if (!recording || !registeredHotkey) return;
    setHotkeyEnabled(registeredHotkey, false).catch((e) =>
      console.error("hotkey pause failed:", e),
    );
    return () => {
      setHotkeyEnabled(registeredHotkey, true).catch((e) =>
        console.error("hotkey resume failed:", e),
      );
    };
  }, [recording, registeredHotkey]);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      if (MODIFIER_KEYS.has(e.key)) return; // still waiting for the real key
      const mods = MODS.filter(([, active]) => active(e)).map(([name]) => name);
      const reason = rejectReason(mods, e.code);
      if (reason) {
        setRejected(reason); // stay in recording mode — try another combination
        return;
      }
      setRejected(null);
      onChange([...mods, e.code].join("+"));
      setRecording(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  const exit = () => {
    setRecording(false);
    setRejected(null);
  };

  return (
    <button
      ref={ref}
      type="button"
      className={`hotkey-input${recording ? " recording" : ""}${rejected ? " rejected" : ""}`}
      title="Click to record a new hotkey"
      onClick={() => {
        if (recording) {
          exit();
        } else {
          setRejected(null);
          setRecording(true);
        }
        ref.current?.focus();
      }}
      onBlur={exit}
    >
      {recording
        ? rejected
          ? `✕ ${rejected} — try again`
          : "Press keys… (esc cancels)"
        : value || "Not set — click to record"}
    </button>
  );
}
