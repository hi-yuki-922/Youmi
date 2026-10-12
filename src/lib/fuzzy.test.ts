import { describe, expect, it } from "vitest";

import { fuzzyScore } from "./fuzzy";

describe("fuzzyScore", () => {
  it("大小写不敏感", () => {
    expect(fuzzyScore("json", "JSON 工具")).not.toBeNull();
    expect(fuzzyScore("JSON", "json")).not.toBeNull();
  });

  it("子序列即可命中", () => {
    expect(fuzzyScore("sn", "JSON 工具")).not.toBeNull();
    expect(fuzzyScore("文d", "文本 diff")).not.toBeNull();
  });

  it("不构成子序列返回 null", () => {
    expect(fuzzyScore("xyz", "JSON 工具")).toBeNull();
    expect(fuzzyScore("diff", "密码本")).toBeNull();
  });

  it("空查询视为全匹配、无加分", () => {
    expect(fuzzyScore("", "任意")).toBe(0);
  });

  it("前缀命中强于中间命中", () => {
    expect(fuzzyScore("j", "json") ?? -Infinity).toBeGreaterThan(
      fuzzyScore("j", "打开 json") ?? -Infinity,
    );
  });

  it("连续命中强于分散命中", () => {
    expect(fuzzyScore("js", "json") ?? -Infinity).toBeGreaterThan(
      fuzzyScore("js", "j something") ?? -Infinity,
    );
  });
});
