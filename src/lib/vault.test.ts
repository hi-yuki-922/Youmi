import { describe, expect, it } from "vitest";

import { clampLockTimeoutMinutes } from "./vault";

describe("clampLockTimeoutMinutes", () => {
  it("0 保留为永不自动锁定", () => {
    expect(clampLockTimeoutMinutes(0)).toBe(0);
  });

  it("小数向上取整，0.5 分钟归 1 分钟而不是被误归为永不", () => {
    expect(clampLockTimeoutMinutes(0.5)).toBe(1);
    expect(clampLockTimeoutMinutes(5.2)).toBe(6);
  });

  it("负数取 0（永不），超上限钳到 1440", () => {
    expect(clampLockTimeoutMinutes(-3)).toBe(0);
    expect(clampLockTimeoutMinutes(2000)).toBe(1440);
  });

  it("区间内的整数原样保留", () => {
    expect(clampLockTimeoutMinutes(5)).toBe(5);
    expect(clampLockTimeoutMinutes(1440)).toBe(1440);
  });
});
