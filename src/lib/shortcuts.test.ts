import { describe, expect, it } from "vitest";

import { formatCombo, hasRequiredModifier } from "@/lib/shortcuts";

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
