import { describe, expect, it } from "vitest";

import { insertAtSelection } from "./palette";

describe("insertAtSelection", () => {
  it("在光标处插入文本", () => {
    expect(insertAtSelection("/t ", "你好", 3, 3)).toEqual({ query: "/t 你好", caret: 5 });
  });

  it("光标未知时追加到末尾", () => {
    expect(insertAtSelection("/t ", "你好", null, null)).toEqual({ query: "/t 你好", caret: 5 });
  });

  it("替换选中的文本", () => {
    expect(insertAtSelection("/t 旧文本", "新", 3, 6)).toEqual({ query: "/t 新", caret: 4 });
  });

  it("反向选区（start 大于 end）同样成立", () => {
    expect(insertAtSelection("/t 旧文本", "新", 6, 3)).toEqual({ query: "/t 新", caret: 4 });
  });

  it("越界下标按字符串长度截住", () => {
    expect(insertAtSelection("ab", "X", 10, 10)).toEqual({ query: "abX", caret: 3 });
  });
});
