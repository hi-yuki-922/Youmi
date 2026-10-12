import { describe, expect, it } from "vitest";

import { evaluate, formatResult, isCalculatable } from "./core";

describe("evaluate", () => {
  it("四则运算与优先级", () => {
    expect(evaluate("1+2")).toEqual({ ok: true, value: 3 });
    expect(evaluate("2+3*4")).toEqual({ ok: true, value: 14 });
    expect(evaluate("10/4")).toEqual({ ok: true, value: 2.5 });
    expect(evaluate("10-4-3")).toEqual({ ok: true, value: 3 });
    expect(evaluate("2*3/6")).toEqual({ ok: true, value: 1 });
  });

  it("括号改变优先级", () => {
    expect(evaluate("(2+3)*4")).toEqual({ ok: true, value: 20 });
    expect(evaluate("((1+2))*((3))")).toEqual({ ok: true, value: 9 });
  });

  it("一元正负号", () => {
    expect(evaluate("-3+5")).toEqual({ ok: true, value: 2 });
    expect(evaluate("2*-3")).toEqual({ ok: true, value: -6 });
    // 运算符后跟一元符号与 "1+-2" 同构，语法上一致地接受
    expect(evaluate("1++2")).toEqual({ ok: true, value: 3 });
    // 一元符号至多连续一个，连打按误输入拒绝
    expect(evaluate("--3")).toEqual({ ok: false, message: "算式不完整" });
    expect(evaluate("1+--2")).toEqual({ ok: false, message: "算式不完整" });
  });

  it("小数与空白", () => {
    expect(evaluate("0.5*4")).toEqual({ ok: true, value: 2 });
    expect(evaluate(".5+1")).toEqual({ ok: true, value: 1.5 });
    expect(evaluate(" 1 + 2 ")).toEqual({ ok: true, value: 3 });
  });

  it("× ÷ 与全角括号按乘除和括号处理", () => {
    expect(evaluate("3×4÷2")).toEqual({ ok: true, value: 6 });
    expect(evaluate("（1+2）×3")).toEqual({ ok: true, value: 9 });
  });

  it("非法输入返回失败并带原因", () => {
    expect(evaluate("")).toEqual({ ok: false, message: "算式不完整" });
    expect(evaluate("*3")).toEqual({ ok: false, message: "算式不完整" });
    expect(evaluate("()")).toEqual({ ok: false, message: "算式不完整" });
    expect(evaluate("1+")).toEqual({ ok: false, message: "算式不完整" });
    expect(evaluate("(1+2")).toEqual({ ok: false, message: "括号不配对" });
    expect(evaluate("1+2)")).toEqual({ ok: false, message: "算式无法完整解析" });
    expect(evaluate("1 2")).toEqual({ ok: false, message: "算式无法完整解析" });
    expect(evaluate("1.2.3")).toEqual({ ok: false, message: "算式无法完整解析" });
    expect(evaluate("3%2")).toEqual({ ok: false, message: "存在无法识别的字符「%」" });
    expect(evaluate("json")).toEqual({ ok: false, message: "存在无法识别的字符「j」" });
  });

  it("除零与非有限结果视为失败", () => {
    expect(evaluate("1/0")).toEqual({ ok: false, message: "除数不能为零" });
    expect(evaluate("1/（0）")).toEqual({ ok: false, message: "除数不能为零" });
    const huge = "9".repeat(400);
    expect(evaluate(`${huge}*${huge}`)).toEqual({
      ok: false,
      message: "结果超出可表示范围",
    });
  });
});

describe("isCalculatable", () => {
  it("含数字与运算符才算算式", () => {
    expect(isCalculatable("1+2")).toBe(true);
    expect(isCalculatable("-3")).toBe(true);
    expect(isCalculatable("×3")).toBe(true);
    expect(isCalculatable("123")).toBe(false);
    expect(isCalculatable("json")).toBe(false);
    // 文本查询常含连字符，不含数字时不按算式处理
    expect(isCalculatable("json-工具")).toBe(false);
    expect(isCalculatable("")).toBe(false);
  });
});

describe("formatResult", () => {
  it("消去浮点噪声", () => {
    expect(formatResult(0.1 + 0.2)).toBe("0.3");
    expect(formatResult(1 / 3)).toBe("0.333333333333");
  });

  it("常规数值原样显示", () => {
    expect(formatResult(2.5)).toBe("2.5");
    expect(formatResult(100)).toBe("100");
    expect(formatResult(-0)).toBe("0");
  });

  it("超大数回退到科学计数法", () => {
    expect(formatResult(1e21)).toBe("1e+21");
  });
});
