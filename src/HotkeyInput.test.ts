import { describe, expect, it } from "vitest";
import { rejectReason } from "./HotkeyInput";

describe("rejectReason", () => {
  it("rejects bare keys", () => {
    expect(rejectReason([], "KeyA")).toBe("needs a modifier");
    expect(rejectReason([], "Digit1")).toBe("needs a modifier");
  });

  it("rejects macOS-reserved combinations", () => {
    expect(rejectReason(["Cmd"], "Tab")).toBe("reserved by macOS");
    expect(rejectReason(["Cmd", "Shift"], "Tab")).toBe("reserved by macOS");
    expect(rejectReason(["Cmd"], "Space")).toBe("reserved by macOS");
    expect(rejectReason(["Ctrl"], "Space")).toBe("reserved by macOS");
  });

  it("rejects Shift alone", () => {
    expect(rejectReason(["Shift"], "KeyA")).toBe("Shift alone just types the key");
    expect(rejectReason(["Shift"], "Space")).toBe("Shift alone just types the key");
  });

  it("rejects Cmd/Alt alone with text keys", () => {
    expect(rejectReason(["Cmd"], "KeyC")).toMatch(/belongs to apps/);
    expect(rejectReason(["Cmd"], "Digit1")).toMatch(/belongs to apps/);
    expect(rejectReason(["Cmd"], "Backquote")).toMatch(/belongs to apps/);
    expect(rejectReason(["Alt"], "KeyE")).toMatch(/belongs to apps/);
  });

  it("accepts two-modifier combinations and non-text keys", () => {
    expect(rejectReason(["Cmd", "Shift"], "Space")).toBeNull(); // the default hotkey
    expect(rejectReason(["Cmd", "Shift"], "KeyA")).toBeNull();
    expect(rejectReason(["Ctrl", "Alt"], "KeyD")).toBeNull();
    expect(rejectReason(["Cmd"], "F5")).toBeNull();
    expect(rejectReason(["Cmd"], "Up")).toBeNull();
    expect(rejectReason(["Ctrl"], "KeyD")).toBeNull();
    expect(rejectReason(["Alt"], "Space")).toBeNull();
  });
});
