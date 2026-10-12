import { describe, expect, it } from "vitest";

import { searchApps, type AppEntry } from "./apps";

const app = (name: string, path = `C:\\lnk\\${name}.lnk`): AppEntry => ({ name, path });

const NAMES = ["微信", "Notepad", "Visual Studio Code", "网易云音乐", "记事本"];

const namedApps = (): AppEntry[] => NAMES.map((name) => app(name));

describe("searchApps", () => {
  it("空查询不返回应用（空查询下面板只展示 Youmi 工具）", () => {
    expect(searchApps(namedApps(), "")).toEqual([]);
  });

  it("英文名片段命中，大小写不敏感", () => {
    const hits = searchApps(namedApps(), "note");
    expect(hits.map((a) => a.name)).toEqual(["Notepad"]);
  });

  it("中文名片段命中", () => {
    const hits = searchApps(namedApps(), "微信");
    expect(hits.map((a) => a.name)).toEqual(["微信"]);
  });

  it("子序列即可命中", () => {
    const hits = searchApps(namedApps(), "vsc");
    expect(hits.map((a) => a.name)).toEqual(["Visual Studio Code"]);
  });

  it("未命中全部过滤", () => {
    expect(searchApps(namedApps(), "firefox")).toEqual([]);
  });

  it("前缀命中强于中间命中", () => {
    const apps = [app("记事本"), app("云记")];
    expect(searchApps(apps, "记")[0]?.name).toBe("记事本");
  });

  it("同分按名称排序保持稳定输出", () => {
    const hits = searchApps([app("b 应用"), app("a 应用")], "应用");
    expect(hits.map((a) => a.name)).toEqual(["a 应用", "b 应用"]);
  });

  it("结果数有上限", () => {
    const apps = Array.from({ length: 30 }, (_, i) => app(`demo 应用 ${i}`));
    expect(searchApps(apps, "demo")).toHaveLength(8);
  });
});
