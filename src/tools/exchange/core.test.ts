import { describe, expect, it } from "vitest";

import {
  convert,
  CURRENCIES,
  currencyName,
  formatAmount,
  formatUpdatedAt,
  isStale,
  type RateSnapshot,
} from "./core";

const RATES: Record<string, number> = { USD: 1, CNY: 7.2, EUR: 0.9, JPY: 150 };

const snapshotAt = (fetchedAt: number): RateSnapshot => ({
  rates: RATES,
  updatedAt: 1000,
  fetchedAt,
});

describe("convert", () => {
  it("USD 基准两段折算（CNY → EUR）", () => {
    const result = convert(72, "CNY", "EUR", RATES);
    expect(result).toEqual({ ok: true, value: (72 / 7.2) * 0.9 });
  });

  it("同币种换算原样返回", () => {
    expect(convert(5, "USD", "USD", RATES)).toEqual({ ok: true, value: 5 });
  });

  it("换算可逆：去与回抵消", () => {
    const there = convert(100, "CNY", "JPY", RATES);
    expect(there.ok).toBe(true);
    if (!there.ok) return;
    const back = convert(there.value, "JPY", "CNY", RATES);
    expect(back.ok).toBe(true);
    if (!back.ok) return;
    expect(back.value).toBeCloseTo(100, 10);
  });

  it("金额非数字报错", () => {
    expect(convert(Number.NaN, "USD", "CNY", RATES).ok).toBe(false);
    expect(convert(Number.POSITIVE_INFINITY, "USD", "CNY", RATES).ok).toBe(false);
  });

  it("缺汇率（源或目标）报错并指明币种", () => {
    const missingSource = convert(1, "GBP", "CNY", RATES);
    expect(missingSource).toEqual({ ok: false, message: "缺少 GBP 的汇率" });
    const missingTarget = convert(1, "USD", "GBP", RATES);
    expect(missingTarget).toEqual({ ok: false, message: "缺少 GBP 的汇率" });
  });
});

describe("isStale", () => {
  it("无快照即过期", () => {
    expect(isStale(null, 1000)).toBe(true);
  });

  it("6 小时内新鲜，超过过期（按 fetchedAt）", () => {
    expect(isStale(snapshotAt(1000), 1000 + 6 * 3600 - 1)).toBe(false);
    expect(isStale(snapshotAt(1000), 1000 + 6 * 3600)).toBe(true);
  });

  it("数据源更新时间晚于拉取时间不影响新鲜度判断", () => {
    // updatedAt 是数据源语义（每日更新），新鲜度只看 fetchedAt
    const snapshot: RateSnapshot = { rates: RATES, updatedAt: 999999, fetchedAt: 1000 };
    expect(isStale(snapshot, 1000 + 3600)).toBe(false);
  });
});

describe("formatAmount", () => {
  it("千分位分组", () => {
    expect(formatAmount(1234567.89)).toBe("1,234,567.89");
  });

  it("最多 4 位小数，多余的四舍五入", () => {
    expect(formatAmount(0.123456)).toBe("0.1235");
    expect(formatAmount(150)).toBe("150");
  });
});

describe("formatUpdatedAt", () => {
  it("unix 秒转本地时间", () => {
    // 固定时刻：2026-10-19 00:30 UTC → 本地时区渲染（只断言格式与回显一致）
    const unix = Date.UTC(2026, 9, 19, 0, 30) / 1000;
    const rendered = formatUpdatedAt(unix);
    expect(rendered).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
    const [datePart, timePart] = rendered.split(" ");
    expect(datePart).toBe(
      `${new Date(unix * 1000).getFullYear()}` +
        `-${String(new Date(unix * 1000).getMonth() + 1).padStart(2, "0")}` +
        `-${String(new Date(unix * 1000).getDate()).padStart(2, "0")}`,
    );
    expect(timePart).toBe(
      `${String(new Date(unix * 1000).getHours()).padStart(2, "0")}` +
        `:${String(new Date(unix * 1000).getMinutes()).padStart(2, "0")}`,
    );
  });
});

describe("币种清单", () => {
  it("按代码排序且唯一", () => {
    const codes = CURRENCIES.map((currency) => currency.code);
    expect([...codes].sort()).toEqual(codes);
    expect(new Set(codes).size).toBe(codes.length);
  });

  it("常用币种在列且中文名到位", () => {
    expect(currencyName("CNY")).toBe("人民币");
    expect(currencyName("USD")).toBe("美元");
    expect(currencyName("JPY")).toBe("日元");
    expect(currencyName("EUR")).toBe("欧元");
  });

  it("未收录代码回退代码本身", () => {
    expect(currencyName("XXX")).toBe("XXX");
  });
});
