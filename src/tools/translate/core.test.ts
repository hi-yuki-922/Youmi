import { describe, expect, it } from "vitest";

import { buildTranslatePrompt, isTranslatable, TRANSLATE_DIRECTIONS } from "./core";

describe("TRANSLATE_DIRECTIONS", () => {
  it("首项为默认方向自动→中文，标签唯一", () => {
    expect(TRANSLATE_DIRECTIONS[0]?.id).toBe("auto-to-zh");
    expect(new Set(TRANSLATE_DIRECTIONS.map((direction) => direction.label)).size).toBe(
      TRANSLATE_DIRECTIONS.length,
    );
  });
});

describe("buildTranslatePrompt", () => {
  it("原文作为 user 消息原样传递", () => {
    const { prompt } = buildTranslatePrompt("hello world", "auto-to-zh");
    expect(prompt).toBe("hello world");
  });

  it("自动方向：检测源语言，中文源文改译英文避免中→中空转", () => {
    const { system } = buildTranslatePrompt("你好", "auto-to-zh");
    expect(system).toContain("Detect the source language");
    expect(system).toContain("Simplified Chinese");
    expect(system).toContain("English");
  });

  it("固定方向只译指定语言对", () => {
    const zhToEn = buildTranslatePrompt("你好", "zh-to-en");
    expect(zhToEn.system).toContain("from Chinese into English");
    expect(zhToEn.system).not.toContain("Detect the source language");

    const enToZh = buildTranslatePrompt("hello", "en-to-zh");
    expect(enToZh.system).toContain("from English into Simplified Chinese");
    expect(enToZh.system).not.toContain("Detect the source language");
  });

  it("系统提示要求只输出译文", () => {
    const { system } = buildTranslatePrompt("hello", "auto-to-zh");
    expect(system).toMatch(/ONLY/i);
    expect(system).toContain("no explanations");
  });
});

describe("isTranslatable", () => {
  it("空白文本不可翻译", () => {
    expect(isTranslatable("hello")).toBe(true);
    expect(isTranslatable("")).toBe(false);
    expect(isTranslatable("  \r\n ")).toBe(false);
  });
});
