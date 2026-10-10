/** JSON 工具核心：纯函数，无 DOM / Tauri 依赖。 */

export type JsonIndent = 2 | 4 | "tab";

export type JsonSuccess<T> = { ok: true; value: T };
export type JsonFailure = { ok: false; message: string; line?: number; column?: number };
export type JsonResult<T> = JsonSuccess<T> | JsonFailure;

const JSON_WS = " \t\n\r";
const MAX_DEPTH = 5000;

/**
 * 定位 JSON 语法错误的字符 offset。
 * 新版 V8 的 "Unexpected token" 类错误消息不含 position，只能自行扫描。
 * 返回 undefined 表示扫描器认为输入合法（与 JSON.parse 判定矛盾时降级为无位置）。
 */
const locateSyntaxError = (input: string): number | undefined => {
  const n = input.length;
  let pos = 0;
  let depth = 0;

  const skipWs = () => {
    while (pos < n && JSON_WS.includes(input[pos]!)) pos++;
  };

  /** @returns -1 表示正常结束，否则返回出错 offset */
  const scanString = (): number => {
    // 前置条件：input[pos] === '"'
    pos++;
    for (;;) {
      if (pos >= n) return pos; // 字符串未闭合
      const ch = input[pos]!;
      if (ch === '"') {
        pos++;
        return -1;
      }
      if (ch === "\\") {
        pos++;
        if (pos >= n) return pos;
        const esc = input[pos]!;
        if (esc === "u") {
          pos++;
          for (let i = 0; i < 4; i++) {
            if (pos >= n || !/[0-9a-fA-F]/.test(input[pos]!)) return pos;
            pos++;
          }
        } else if (
          esc !== '"' &&
          esc !== "\\" &&
          esc !== "/" &&
          esc !== "b" &&
          esc !== "f" &&
          esc !== "n" &&
          esc !== "r" &&
          esc !== "t"
        ) {
          return pos; // 非法转义
        } else {
          pos++;
        }
      } else if (ch < " ") {
        return pos; // 未转义的控制字符
      } else {
        pos++;
      }
    }
  };

  /** @returns -1 表示正常结束，否则返回出错 offset */
  const scanNumber = (): number => {
    // 前置条件：input[pos] === '-' 或数字
    if (input[pos] === "-") pos++;
    if (pos >= n) return pos;
    if (input[pos] === "0") {
      pos++;
    } else if (/[1-9]/.test(input[pos]!)) {
      while (pos < n && /[0-9]/.test(input[pos]!)) pos++;
    } else {
      return pos; // '-' 后无数字
    }
    if (input[pos] === ".") {
      pos++;
      if (pos >= n || !/[0-9]/.test(input[pos]!)) return pos;
      while (pos < n && /[0-9]/.test(input[pos]!)) pos++;
    }
    if (input[pos] === "e" || input[pos] === "E") {
      pos++;
      if (input[pos] === "+" || input[pos] === "-") pos++;
      if (pos >= n || !/[0-9]/.test(input[pos]!)) return pos;
      while (pos < n && /[0-9]/.test(input[pos]!)) pos++;
    }
    return -1;
  };

  const scanLiteral = (word: string): number => {
    if (input.startsWith(word, pos)) {
      pos += word.length;
      return -1;
    }
    return pos;
  };

  /** @returns -1 表示正常结束，否则返回出错 offset */
  const scanValue = (): number => {
    skipWs();
    if (pos >= n) return pos; // 意外结束
    const ch = input[pos]!;

    if (ch === '"') return scanString();

    if (ch === "{") {
      depth++;
      if (depth > MAX_DEPTH) return pos;
      pos++;
      skipWs();
      if (input[pos] === "}") {
        pos++;
        depth--;
        return -1;
      }
      for (;;) {
        skipWs();
        if (pos >= n || input[pos] !== '"') return pos; // 期望字符串 key
        let r = scanString();
        if (r !== -1) return r;
        skipWs();
        if (input[pos] !== ":") return pos;
        pos++;
        r = scanValue();
        if (r !== -1) return r;
        skipWs();
        if (input[pos] === ",") {
          pos++;
          continue;
        }
        if (input[pos] === "}") {
          pos++;
          depth--;
          return -1;
        }
        return pos;
      }
    }

    if (ch === "[") {
      depth++;
      if (depth > MAX_DEPTH) return pos;
      pos++;
      skipWs();
      if (input[pos] === "]") {
        pos++;
        depth--;
        return -1;
      }
      for (;;) {
        const r = scanValue();
        if (r !== -1) return r;
        skipWs();
        if (input[pos] === ",") {
          pos++;
          continue;
        }
        if (input[pos] === "]") {
          pos++;
          depth--;
          return -1;
        }
        return pos;
      }
    }

    if (ch === "t") return scanLiteral("true");
    if (ch === "f") return scanLiteral("false");
    if (ch === "n") return scanLiteral("null");
    if (ch === "-" || (ch >= "0" && ch <= "9")) return scanNumber();
    return pos; // 意外 token
  };

  const r = scanValue();
  if (r !== -1) return r;
  skipWs();
  if (pos < n) return pos; // 顶层值后有多余内容
  return undefined;
};

const withPosition = (input: string, offset: number, message: string): JsonFailure => {
  const before = input.slice(0, offset);
  const line = before.split("\n").length;
  const column = offset - before.lastIndexOf("\n");
  return { ok: false, message, line, column };
};

const toFailure = (input: string, error: unknown): JsonFailure => {
  const message = error instanceof Error ? error.message : String(error);
  const position = /position (\d+)/.exec(message);
  if (position) {
    return withPosition(input, Number(position[1]), message);
  }
  const offset = locateSyntaxError(input);
  if (offset === undefined) return { ok: false, message };
  return withPosition(input, offset, message);
};

export const formatJson = (input: string, indent: JsonIndent): JsonResult<string> => {
  try {
    const value = JSON.parse(input);
    const indentValue = indent === "tab" ? "\t" : indent;
    return { ok: true, value: JSON.stringify(value, null, indentValue) };
  } catch (error) {
    return toFailure(input, error);
  }
};

export const minifyJson = (input: string): JsonResult<string> => {
  try {
    const value = JSON.parse(input);
    return { ok: true, value: JSON.stringify(value) };
  } catch (error) {
    return toFailure(input, error);
  }
};

export type JsonValidation = { ok: true } | JsonFailure;

export const validateJson = (input: string): JsonValidation => {
  try {
    JSON.parse(input);
    return { ok: true };
  } catch (error) {
    return toFailure(input, error);
  }
};

/** 任意文本 → 带引号的 JSON 字符串字面量（转义）。恒成功。 */
export const escapeToJsonString = (text: string): string => JSON.stringify(text);

/** JSON 字符串字面量 → 原文（去转义）。非字符串字面量则报错。 */
export const unescapeFromJsonString = (literal: string): JsonResult<string> => {
  try {
    const value = JSON.parse(literal);
    if (typeof value !== "string") {
      return { ok: false, message: "输入不是 JSON 字符串字面量（应为带引号的字符串）" };
    }
    return { ok: true, value };
  } catch (error) {
    return toFailure(literal, error);
  }
};
