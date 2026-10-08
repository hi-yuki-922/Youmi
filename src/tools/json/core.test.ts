import { describe, expect, it } from "vitest";

import {
  escapeToJsonString,
  formatJson,
  minifyJson,
  unescapeFromJsonString,
  validateJson,
} from "./core";

describe("formatJson", () => {
  it("用 2 空格缩进格式化压缩过的 JSON", () => {
    const result = formatJson('{"name":"Youmi","tags":["a","b"]}', 2);
    expect(result).toEqual({
      ok: true,
      value: '{\n  "name": "Youmi",\n  "tags": [\n    "a",\n    "b"\n  ]\n}',
    });
  });

  it("用 4 空格缩进格式化", () => {
    const result = formatJson('{"a":1}', 4);
    expect(result).toEqual({
      ok: true,
      value: '{\n    "a": 1\n}',
    });
  });

  it("用 tab 缩进格式化", () => {
    const result = formatJson('{"a": 1}', "tab");
    expect(result).toEqual({
      ok: true,
      value: '{\n\t"a": 1\n}',
    });
  });

  it("畸形 JSON 报错并定位行列（V8 消息不带 position 的场景）", () => {
    const result = formatJson('{\n  "a": 1,\n  "b": ,\n}', 2);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.line).toBe(3);
      expect(result.column).toBe(8);
      expect(result.message.length).toBeGreaterThan(0);
    }
  });

  it("畸形 JSON 报错并定位行列（V8 消息带 position 的场景）", () => {
    const result = formatJson('{"a": 1', 2);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.line).toBe(1);
      expect(result.column).toBe(8);
    }
  });
});

describe("minifyJson", () => {
  it("压缩已格式化的 JSON 为最小体积", () => {
    const result = minifyJson('{\n  "name": "Youmi",\n  "tags": [\n    "a"\n  ]\n}');
    expect(result).toEqual({
      ok: true,
      value: '{"name":"Youmi","tags":["a"]}',
    });
  });
});

describe("validateJson", () => {
  it("合法 JSON 通过校验", () => {
    expect(validateJson('{"a": [1, 2], "b": "文本"}')).toEqual({ ok: true });
  });

  it("尾随逗号报错并定位行列", () => {
    const result = validateJson('{\n  "a": 1,\n}');
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.line).toBe(3);
      expect(result.column).toBe(1);
    }
  });
});

describe("escapeToJsonString", () => {
  it("把任意文本转成带引号的 JSON 字符串字面量", () => {
    expect(escapeToJsonString('{"a": 1}')).toBe('"{\\"a\\": 1}"');
  });

  it("转义换行与中文保留原文", () => {
    expect(escapeToJsonString("中文\n换行")).toBe('"中文\\n换行"');
  });
});

describe("unescapeFromJsonString", () => {
  it("把 JSON 字符串字面量还原为原文", () => {
    const result = unescapeFromJsonString('"{\\"a\\": 1}"');
    expect(result).toEqual({ ok: true, value: '{"a": 1}' });
  });

  it("非字符串字面量报错", () => {
    const result = unescapeFromJsonString("{a}");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.message.length).toBeGreaterThan(0);
    }
  });
});
