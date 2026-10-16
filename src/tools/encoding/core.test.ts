import { describe, expect, it } from "vitest";

import {
  base64ToText,
  dataUriToSvg,
  normalizeIconName,
  paramsToQuery,
  parseQuery,
  svgIconScss,
  svgToDataUri,
  textToBase64,
} from "./core";

describe("svgToDataUri", () => {
  it("SVG 源码转成可直接用于 CSS background-image 的 data URI", () => {
    const svg = '<svg width="8" height="8"><rect fill="#FF0000" width="8"/></svg>';
    expect(svgToDataUri(svg)).toBe(
      "data:image/svg+xml,%3Csvg width=%228%22 height=%228%22%3E%3Crect fill=%22%23FF0000%22 width=%228%22/%3E%3C/svg%3E",
    );
  });

  it("中文与 emoji 按 UTF-8 百分号编码", () => {
    expect(svgToDataUri("<svg><text>中😀</text></svg>")).toBe(
      "data:image/svg+xml,%3Csvg%3E%3Ctext%3E%E4%B8%AD%F0%9F%98%80%3C/text%3E%3C/svg%3E",
    );
  });

  it("编码引号防外层属性破坏；空格、括号、斜杠保持可读", () => {
    expect(svgToDataUri(`<svg>it's (ok)</svg>`)).toBe(
      "data:image/svg+xml,%3Csvg%3Eit%27s (ok)%3C/svg%3E",
    );
  });

  it("空输入只返回前缀", () => {
    expect(svgToDataUri("")).toBe("data:image/svg+xml,");
  });
});

describe("dataUriToSvg", () => {
  it("带 data URI 前缀的输入解码回 SVG 源码", () => {
    const result = dataUriToSvg(
      "data:image/svg+xml,%3Csvg%3E%3Ctext%3E%E4%B8%AD%F0%9F%98%80%3C/text%3E%3C/svg%3E",
    );
    expect(result).toEqual({ ok: true, value: "<svg><text>中😀</text></svg>" });
  });

  it("兼容 ;utf8 前缀变体", () => {
    expect(dataUriToSvg("data:image/svg+xml;utf8,%3Csvg%3E")).toEqual({
      ok: true,
      value: "<svg>",
    });
  });

  it("不带前缀的裸编码文本也能解码", () => {
    expect(dataUriToSvg("%3Csvg%3E%E4%B8%AD%3C/svg%3E")).toEqual({
      ok: true,
      value: "<svg>中</svg>",
    });
  });

  it("非法百分号序列报错", () => {
    const result = dataUriToSvg("data:image/svg+xml,%3Csvg%ZZ");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.message.length).toBeGreaterThan(0);
    }
  });

  it("空输入返回空字符串", () => {
    expect(dataUriToSvg("")).toEqual({ ok: true, value: "" });
  });

  it("编码解码 roundtrip 还原中文、emoji 与引号", () => {
    const svg = '<svg xmlns="http://www.w3.org/2000/svg">中文 "引号" & 😀</svg>';
    const result = dataUriToSvg(svgToDataUri(svg));
    expect(result).toEqual({ ok: true, value: svg });
  });
});

describe("normalizeIconName", () => {
  it("空值与纯空白回退默认名 icon", () => {
    expect(normalizeIconName("")).toBe("icon");
    expect(normalizeIconName("   ")).toBe("icon");
  });

  it("空白折叠为连字符", () => {
    expect(normalizeIconName("my icon")).toBe("my-icon");
    expect(normalizeIconName("  spaced  name ")).toBe("spaced-name");
  });

  it("类名非法字符替换为连字符（含中文字符）", () => {
    expect(normalizeIconName("a.b/c!")).toBe("a-b-c-");
    expect(normalizeIconName("箭头")).toBe("--");
  });

  it("数字开头补 icon- 前缀，避免非法 CSS 类名", () => {
    expect(normalizeIconName("42icon")).toBe("icon-42icon");
  });

  it("合法名称原样保留", () => {
    expect(normalizeIconName("arrow-left_2")).toBe("arrow-left_2");
  });
});

describe("svgIconScss", () => {
  it("生成 iconify 风格片段：@include icon 固定，--svg 指向 data URI", () => {
    const dataUri = svgToDataUri('<svg width="8" height="8"/>');
    expect(svgIconScss(dataUri, "menu")).toBe(
      `.menu {\n  @include icon;\n  --svg: url("${dataUri}");\n}`,
    );
  });

  it("空 iconName 注入默认名 icon", () => {
    expect(svgIconScss("data:image/svg+xml,%3Csvg%3E", "")).toBe(
      '.icon {\n  @include icon;\n  --svg: url("data:image/svg+xml,%3Csvg%3E");\n}',
    );
  });

  it("特殊字符的 iconName 规范化后注入，不破坏 scss 结构", () => {
    const template = svgIconScss("data:image/svg+xml,x", "my icon!");
    expect(template).toBe(
      '.my-icon- {\n  @include icon;\n  --svg: url("data:image/svg+xml,x");\n}',
    );
  });

  it("data URI 原样嵌入（其中无未转义引号）", () => {
    const dataUri = svgToDataUri(`<svg>it's "q" & more</svg>`);
    expect(svgIconScss(dataUri, "q")).toContain(`--svg: url("${dataUri}");`);
  });
});

describe("textToBase64", () => {
  it("ASCII 文本编码为 Base64", () => {
    expect(textToBase64("hello")).toBe("aGVsbG8=");
  });

  it("中文按 UTF-8 编码", () => {
    expect(textToBase64("你好")).toBe("5L2g5aW9");
  });

  it("emoji（4 字节 UTF-8）编码正确", () => {
    expect(textToBase64("😀")).toBe("8J+YgA==");
  });

  it("空输入返回空字符串", () => {
    expect(textToBase64("")).toBe("");
  });
});

describe("base64ToText", () => {
  it("标准 Base64 解码回文本，容忍缺失的填充", () => {
    expect(base64ToText("5L2g5aW9")).toEqual({ ok: true, value: "你好" });
  });

  it("容忍粘贴时带入的换行与空白", () => {
    expect(base64ToText("5L2g\n5aW9\r\n")).toEqual({ ok: true, value: "你好" });
  });

  it("兼容 base64url 字母表（- 与 _）", () => {
    expect(base64ToText("6KW-")).toEqual({ ok: true, value: "襾" });
    expect(base64ToText("6KW_")).toEqual({ ok: true, value: "西" });
    expect(base64ToText("8J+YgA")).toEqual({ ok: true, value: "😀" });
  });

  it("长度为 4n+1 时报错", () => {
    const result = base64ToText("AAAAA");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.message.length).toBeGreaterThan(0);
    }
  });

  it("含非法字符时报错", () => {
    const result = base64ToText("a@bc");
    expect(result.ok).toBe(false);
  });

  it("解码结果不是合法 UTF-8 时报错", () => {
    expect(base64ToText("//79").ok).toBe(false);
  });

  it("空输入返回空字符串", () => {
    expect(base64ToText("")).toEqual({ ok: true, value: "" });
  });

  it("编码解码 roundtrip 还原中文、emoji 与引号", () => {
    const text = '中文 "引号" \n 换行 😀';
    const result = base64ToText(textToBase64(text));
    expect(result).toEqual({ ok: true, value: text });
  });
});

describe("parseQuery", () => {
  it("解析键值对，保持顺序与重复键", () => {
    expect(parseQuery("a=1&b=2&a=3")).toEqual({
      ok: true,
      value: [
        { key: "a", value: "1" },
        { key: "b", value: "2" },
        { key: "a", value: "3" },
      ],
    });
  });

  it("容忍前导问号，百分号解码为中文", () => {
    expect(parseQuery("?q=%E4%B8%AD%E6%96%87&tab=1")).toEqual({
      ok: true,
      value: [
        { key: "q", value: "中文" },
        { key: "tab", value: "1" },
      ],
    });
  });

  it("无值的键解析为空字符串值", () => {
    expect(parseQuery("flag&x=1")).toEqual({
      ok: true,
      value: [
        { key: "flag", value: "" },
        { key: "x", value: "1" },
      ],
    });
  });

  it("+ 解码为空格", () => {
    expect(parseQuery("q=a+b")).toEqual({ ok: true, value: [{ key: "q", value: "a b" }] });
  });

  it("粘贴完整 URL 时取其 query 部分", () => {
    expect(parseQuery("https://api.example.com/v1?page=2&size=10")).toEqual({
      ok: true,
      value: [
        { key: "page", value: "2" },
        { key: "size", value: "10" },
      ],
    });
  });

  it("无法解析的 URL 报错", () => {
    expect(parseQuery("http://").ok).toBe(false);
  });

  it("空输入返回空表", () => {
    expect(parseQuery("")).toEqual({ ok: true, value: [] });
  });
});

describe("paramsToQuery", () => {
  it("键值表编码为 query 字符串，空格编码为 +", () => {
    expect(
      paramsToQuery([
        { key: "q", value: "a b" },
        { key: "n", value: "1" },
      ]),
    ).toBe("q=a+b&n=1");
  });

  it("中文与特殊字符按百分号编码", () => {
    expect(paramsToQuery([{ key: "k", value: "中文/值" }])).toBe(
      "k=%E4%B8%AD%E6%96%87%2F%E5%80%BC",
    );
  });

  it("重复键与空值保持原样", () => {
    expect(
      paramsToQuery([
        { key: "a", value: "" },
        { key: "a", value: "x" },
      ]),
    ).toBe("a=&a=x");
  });

  it("空表返回空字符串", () => {
    expect(paramsToQuery([])).toBe("");
  });

  it("编码解析 roundtrip 还原键值表", () => {
    const rows = [
      { key: "q", value: "中文 值" },
      { key: "flag", value: "" },
      { key: "q", value: "2" },
    ];
    const result = parseQuery(paramsToQuery(rows));
    expect(result).toEqual({ ok: true, value: rows });
  });
});
