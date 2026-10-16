import { describe, expect, it } from "vitest";

import { hasRequiredModifier } from "@/lib/shortcuts";

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
