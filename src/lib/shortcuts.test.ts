import { describe, expect, it } from "vitest";

import { eventToCombo, formatCombo, hasRequiredModifier } from "@/lib/shortcuts";

const keyEvent = (overrides: Partial<KeyboardEvent>): KeyboardEvent =>
  ({
    code: "KeyV",
    key: "v",
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...overrides,
  }) as KeyboardEvent;

describe("eventToCombo", () => {
  it("collects modifiers in canonical order plus the physical key", () => {
    expect(eventToCombo(keyEvent({ code: "KeyC", ctrlKey: true, shiftKey: true }))).toBe(
      "ctrl+shift+KeyC",
    );
    expect(eventToCombo(keyEvent({ code: "Space", altKey: true, metaKey: true }))).toBe(
      "alt+super+Space",
    );
    expect(eventToCombo(keyEvent({ code: "KeyT", altKey: true }))).toBe("alt+KeyT");
  });

  it("returns null while only modifiers are pressed", () => {
    expect(eventToCombo(keyEvent({ code: "ControlLeft", key: "Control" }))).toBeNull();
    expect(eventToCombo(keyEvent({ code: "AltRight", key: "Alt", altKey: true }))).toBeNull();
    expect(eventToCombo(keyEvent({ code: "MetaLeft", key: "Meta", metaKey: true }))).toBeNull();
  });

  it("keeps modifier-less combos for the backend to reject", () => {
    expect(eventToCombo(keyEvent({ code: "Digit1" }))).toBe("Digit1");
  });
});

describe("hasRequiredModifier", () => {
  it("accepts ctrl, alt and win combos", () => {
    expect(hasRequiredModifier("ctrl+shift+KeyC")).toBe(true);
    expect(hasRequiredModifier("alt+KeyV")).toBe(true);
    expect(hasRequiredModifier("alt+super+Space")).toBe(true);
  });

  it("rejects combos without ctrl, alt or win", () => {
    expect(hasRequiredModifier("shift+KeyV")).toBe(false);
    expect(hasRequiredModifier("KeyV")).toBe(false);
    expect(hasRequiredModifier("F5")).toBe(false);
  });
});

describe("formatCombo", () => {
  it("formats modifiers and known key names", () => {
    expect(formatCombo("alt+KeyV")).toBe("Alt+V");
    expect(formatCombo("alt+super+Space")).toBe("Alt+Win+Space");
    expect(formatCombo("ctrl+shift+Digit1")).toBe("Ctrl+Shift+1");
    expect(formatCombo("ctrl+alt+Semicolon")).toBe("Ctrl+Alt+;");
    expect(formatCombo("ctrl+ArrowUp")).toBe("Ctrl+↑");
  });

  it("passes through numpad and F-key names", () => {
    expect(formatCombo("ctrl+Numpad5")).toBe("Ctrl+Num5");
    expect(formatCombo("alt+F5")).toBe("Alt+F5");
  });
});
