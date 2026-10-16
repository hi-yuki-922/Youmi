import { describe, expect, it } from "vitest";

import { comboSymbols, formatCombo } from "@/lib/kbd";

describe("comboSymbols", () => {
  it("maps modifiers to distinct glyphs (alt ⌘ vs win ⊞)", () => {
    expect(comboSymbols("ctrl+shift+Digit1")).toEqual(["⌃", "⇧", "1"]);
    expect(comboSymbols("alt+KeyV")).toEqual(["⌘", "V"]);
    expect(comboSymbols("super+KeyE")).toEqual(["⊞", "E"]);
  });

  it("renders multi-modifier combos with win first regardless of input order", () => {
    expect(comboSymbols("alt+super+Space")).toEqual(["⊞", "⌘", "Space"]);
    expect(comboSymbols("super+alt+Space")).toEqual(["⊞", "⌘", "Space"]);
  });

  it("maps enter, escape and arrow keys to glyphs", () => {
    expect(comboSymbols("ctrl+Enter")).toEqual(["⌃", "⏎"]);
    expect(comboSymbols("Escape")).toEqual(["⎋"]);
    expect(comboSymbols("ctrl+ArrowUp")).toEqual(["⌃", "↑"]);
    expect(comboSymbols("shift+ArrowDown")).toEqual(["⇧", "↓"]);
  });

  it("keeps letters, digits and punctuation as-is", () => {
    expect(comboSymbols("ctrl+alt+Semicolon")).toEqual(["⌃", "⌘", ";"]);
    expect(comboSymbols("alt+F5")).toEqual(["⌘", "F5"]);
    expect(comboSymbols("ctrl+Numpad5")).toEqual(["⌃", "Num5"]);
  });
});

describe("formatCombo", () => {
  it("formats text labels in the same display order as symbols", () => {
    expect(formatCombo("alt+KeyV")).toBe("Alt+V");
    expect(formatCombo("alt+super+Space")).toBe("Win+Alt+Space");
    expect(formatCombo("ctrl+shift+Digit1")).toBe("Ctrl+Shift+1");
    expect(formatCombo("ctrl+ArrowUp")).toBe("Ctrl+↑");
  });

  it("passes through numpad and F-key names", () => {
    expect(formatCombo("ctrl+Numpad5")).toBe("Ctrl+Num5");
    expect(formatCombo("alt+F5")).toBe("Alt+F5");
  });
});
