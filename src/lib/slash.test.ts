import { describe, expect, it } from "vitest";

import {
  deliverToolText,
  parseSlashQuery,
  subscribeToolText,
  takePendingToolText,
  type SlashCommand,
} from "./slash";

const command = (name: string): SlashCommand => ({
  name,
  description: `${name} 的描述`,
  usage: `/${name} <参数>`,
  run: () => {},
});

const COMMANDS = [command("t"), command("c")];

describe("parseSlashQuery", () => {
  it("非 / 开头不进入斜杠模式", () => {
    expect(parseSlashQuery("json 格式化", COMMANDS)).toEqual({ mode: "none" });
    expect(parseSlashQuery(" /t", COMMANDS)).toEqual({ mode: "none" });
  });

  it("裸斜杠列出全部命令", () => {
    const result = parseSlashQuery("/", COMMANDS);
    expect(result).toEqual({ mode: "list", matches: COMMANDS });
  });

  it("命令名前缀过滤", () => {
    expect(parseSlashQuery("/t", COMMANDS)).toEqual({ mode: "list", matches: [COMMANDS[0]] });
    expect(parseSlashQuery("/c", COMMANDS)).toEqual({ mode: "list", matches: [COMMANDS[1]] });
  });

  it("前缀无命中的名字报未知命令", () => {
    expect(parseSlashQuery("/x", COMMANDS)).toEqual({ mode: "unknown", name: "x" });
    expect(parseSlashQuery("/te", COMMANDS)).toEqual({ mode: "unknown", name: "te" });
  });

  it("无空格的紧凑输入不算命令（/t你好）", () => {
    expect(parseSlashQuery("/t你好", COMMANDS)).toEqual({ mode: "unknown", name: "t你好" });
  });

  it("命令名 + 首个空格 = 执行，其余全是参数", () => {
    expect(parseSlashQuery("/t 你好 世界", COMMANDS)).toEqual({
      mode: "execute",
      command: COMMANDS[0],
      args: "你好 世界",
    });
    expect(parseSlashQuery("/t ", COMMANDS)).toEqual({
      mode: "execute",
      command: COMMANDS[0],
      args: "",
    });
  });

  it("执行形态下命令名必须精确命中", () => {
    expect(parseSlashQuery("/te 你好", COMMANDS)).toEqual({ mode: "unknown", name: "te" });
  });
});

describe("工具页文本直达通道", () => {
  it("页面未挂载（未订阅）时暂存，取走即清", () => {
    deliverToolText("translate", "你好");
    expect(takePendingToolText("translate")).toBe("你好");
    expect(takePendingToolText("translate")).toBeUndefined();
  });

  it("页面已挂载（已订阅）时监听直送，不入暂存", () => {
    const received: string[] = [];
    const unsubscribe = subscribeToolText("translate", (text) => received.push(text));
    deliverToolText("translate", "bonjour");
    expect(received).toEqual(["bonjour"]);
    expect(takePendingToolText("translate")).toBeUndefined();
    unsubscribe();
  });

  it("订阅取消后恢复暂存路径", () => {
    const received: string[] = [];
    const unsubscribe = subscribeToolText("translate", (text) => received.push(text));
    unsubscribe();
    deliverToolText("translate", "hola");
    expect(received).toEqual([]);
    expect(takePendingToolText("translate")).toBe("hola");
  });

  it("不同工具页互不串扰", () => {
    deliverToolText("json", "{}");
    expect(takePendingToolText("translate")).toBeUndefined();
    expect(takePendingToolText("json")).toBe("{}");
  });
});
