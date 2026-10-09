import { describe, expect, it } from "vitest";

import { formatEntryTime } from "./core";

describe("formatEntryTime", () => {
  const now = new Date("2026-10-09T15:30:00");

  it("今天的时间只显示时:分", () => {
    const today = new Date("2026-10-09T09:05:00").getTime();
    expect(formatEntryTime(today, now)).toBe("09:05");
  });

  it("今年早些时候显示 月-日 时:分", () => {
    const thisYear = new Date("2026-03-02T21:08:00").getTime();
    expect(formatEntryTime(thisYear, now)).toBe("03-02 21:08");
  });

  it("更早年份显示完整日期", () => {
    const old = new Date("2024-12-31T23:59:00").getTime();
    expect(formatEntryTime(old, now)).toBe("2024-12-31");
  });
});
