import { describe, expect, it } from "vitest";

import { searchTools, TOOLS } from "./tools";

describe("searchTools", () => {
  it("空查询返回全部工具、保持注册顺序", () => {
    expect(searchTools("").map((t) => t.id)).toEqual(TOOLS.map((t) => t.id));
  });

  it("按名称命中并排序", () => {
    expect(searchTools("json")[0]?.id).toBe("json");
    expect(searchTools("diff")[0]?.id).toBe("diff");
    expect(searchTools("编码")[0]?.id).toBe("encoding");
  });

  it("按关键词命中", () => {
    expect(searchTools("base64")[0]?.id).toBe("encoding");
    expect(searchTools("开机")[0]?.id).toBe("settings");
  });

  it("无命中返回空列表", () => {
    expect(searchTools("zzz不存在")).toEqual([]);
  });
});
