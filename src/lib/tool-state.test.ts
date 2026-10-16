import { describe, expect, it, vi } from "vitest";

import {
  loadToolState,
  restoreNullableString,
  restoreString,
  saveToolState,
  type ToolStateStorage,
} from "./tool-state";

/** 内存存储：与 localStorage 同形（getItem/setItem），供纯函数层测试。 */
const memoryStorage = (initial: Record<string, string> = {}) => {
  const data = new Map(Object.entries(initial));
  const storage: ToolStateStorage = {
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => void data.set(key, value),
  };
  return { storage, data };
};

describe("saveToolState / loadToolState roundtrip", () => {
  it("保存后恢复出相同的状态（嵌套对象）", () => {
    const { storage } = memoryStorage();
    const state = {
      input: '{"a":1}',
      output: '{\n  "a": 1\n}',
      options: { indent: "tab", view: "split" },
    };

    saveToolState(storage, "json.page", state);
    const restored = loadToolState(storage, "json.page", null, (value) => value);

    expect(restored).toEqual(state);
  });

  it("键带 tools. 前缀，按工具划分", () => {
    const { storage, data } = memoryStorage();

    saveToolState(storage, "diff.old", "hello");

    expect([...data.keys()]).toEqual(["tools.diff.old"]);
  });

  it("无存档时回退默认值", () => {
    const { storage } = memoryStorage();

    expect(loadToolState(storage, "json.input", "", restoreString)).toBe("");
  });

  it("存档为空字符串同样如实恢复——清空后重启不复活旧内容", () => {
    const { storage } = memoryStorage();
    saveToolState(storage, "json.input", "旧内容");

    saveToolState(storage, "json.input", "");

    expect(loadToolState(storage, "json.input", "fallback", restoreString)).toBe("");
  });

  it("null 状态经 restoreNullableString 恢复（错误态「无错误」）", () => {
    const { storage } = memoryStorage();
    saveToolState(storage, "encoding.error", null);

    expect(loadToolState<string | null>(storage, "encoding.error", "", restoreNullableString)).toBe(
      null,
    );
  });

  it("JSON 损坏时回退默认值，不抛出", () => {
    const { storage } = memoryStorage({ "tools.json.input": "{not json" });

    expect(loadToolState(storage, "json.input", "fallback", restoreString)).toBe("fallback");
  });

  it("存档形状不符（restore 返回 undefined）时回退默认值", () => {
    // localStorage 只存字符串："42" 解析为数字，形状不符
    const { storage } = memoryStorage({ "tools.json.input": "42" });

    expect(loadToolState(storage, "json.input", "fallback", restoreString)).toBe("fallback");
  });

  it("restore 恢复出 undefined 视为形状不符，回退默认值", () => {
    const { storage } = memoryStorage({ "tools.json.indent": "3" });

    expect(
      loadToolState(storage, "json.indent", 2, (value) =>
        value === 2 || value === 4 || value === "tab" ? (value as 2 | 4 | "tab") : undefined,
      ),
    ).toBe(2);
  });

  it("保存失败（存储抛错）不向外抛出", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    const storage: ToolStateStorage = {
      getItem: () => null,
      setItem: () => {
        throw new Error("quota exceeded");
      },
    };

    expect(() => saveToolState(storage, "json.input", "x")).not.toThrow();
    error.mockRestore();
  });
});
