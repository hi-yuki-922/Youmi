/** 编码转换核心：纯函数，无 DOM / Tauri 依赖。 */

export type ConvertResult<T> = { ok: true; value: T } | { ok: false; message: string };

const SVG_DATA_URI_PREFIX = "data:image/svg+xml,";

/**
 * 非打印 ASCII 字符统一按 UTF-8 百分号编码（大写十六进制）。
 * 用 TextEncoder 而非 encodeURIComponent，保证孤立代理项不抛错。
 */
const percentEncodeUtf8 = (text: string): string => {
  let out = "";
  for (const byte of new TextEncoder().encode(text)) {
    out += `%${byte.toString(16).toUpperCase().padStart(2, "0")}`;
  }
  return out;
};

/**
 * SVG 源码 → data URI。恒成功。
 * 只编码会破坏 CSS url()/HTML 属性语境的字符（% #" '& <>），
 * 其余打印 ASCII 保持可读；非 ASCII 与控制字符按 UTF-8 百分号编码。
 */
export const svgToDataUri = (svg: string): string => {
  const body = svg
    .replace(/%/g, "%25")
    .replace(/#/g, "%23")
    .replace(/&/g, "%26")
    .replace(/"/g, "%22")
    .replace(/'/g, "%27")
    .replace(/</g, "%3C")
    .replace(/>/g, "%3E")
    .replace(/[^\x20-\x7E]+/gu, percentEncodeUtf8);
  return SVG_DATA_URI_PREFIX + body;
};

/** URL 编码（data URI 或裸编码文本）→ SVG 源码。前缀可省略。 */
export const dataUriToSvg = (input: string): ConvertResult<string> => {
  const trimmed = input.trim();
  if (trimmed === "") return { ok: true, value: "" };
  const withPrefix = /^data:image\/svg\+xml[^,]*,/i.exec(trimmed);
  const body = withPrefix ? trimmed.slice(trimmed.indexOf(",") + 1) : trimmed;
  try {
    return { ok: true, value: decodeURIComponent(body) };
  } catch {
    return { ok: false, message: "URL 编码格式不正确（存在非法的百分号序列）" };
  }
};

/** iconName 规范化为 scss 类名：空白折叠为连字符，类名非法字符替换为连字符；空值回退 icon。 */
export const normalizeIconName = (iconName: string): string => {
  const name = iconName
    .trim()
    .replace(/\s+/g, "-")
    .replace(/[^A-Za-z0-9_-]/g, "-");
  if (name === "") return "icon";
  // CSS 类名不能以数字开头
  return /^[0-9]/.test(name) ? `icon-${name}` : name;
};

/** SVG 图标的 iconify 风格 scss 片段：类名内联 @include icon，--svg 指向 data URI。 */
export const svgIconScss = (dataUri: string, iconName: string): string => {
  const name = normalizeIconName(iconName);
  return `.${name} {\n  @include icon;\n  --svg: url("${dataUri}");\n}`;
};

/**
 * 文本 → Base64（UTF-8）。恒成功。
 * 分块拼接二进制串，避免长文本展开参数栈溢出。
 */
export const textToBase64 = (text: string): string => {
  const bytes = new TextEncoder().encode(text);
  const CHUNK = 0x8000;
  let binary = "";
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(binary);
};

/**
 * Base64 → 文本（UTF-8）。
 * 容忍空白、缺失填充与 base64url 字母表（- _）；严格校验 UTF-8，二进制垃圾报错。
 */
export const base64ToText = (input: string): ConvertResult<string> => {
  const compact = input.replace(/\s+/g, "");
  if (compact === "") return { ok: true, value: "" };
  const body = compact.replace(/-/g, "+").replace(/_/g, "/").replace(/=+$/, "");
  if (body.length % 4 === 1) {
    return { ok: false, message: "Base64 长度不合法（去掉填充后长度对 4 取余不能为 1）" };
  }
  const padded = body + "=".repeat((4 - (body.length % 4)) % 4);
  if (!/^[A-Za-z0-9+/]*={0,2}$/.test(padded)) {
    return { ok: false, message: "包含非 Base64 字符" };
  }
  try {
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (ch) => ch.charCodeAt(0));
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    return { ok: true, value: text };
  } catch {
    return { ok: false, message: "不是有效的 Base64 数据，或解码结果不是合法的 UTF-8 文本" };
  }
};

export type QueryParam = { key: string; value: string };

/**
 * URL query 字符串 → 键值表，保持顺序与重复键。
 * 粘贴完整 URL 时取其 query 部分；无法解析的 URL 报错。
 */
export const parseQuery = (input: string): ConvertResult<QueryParam[]> => {
  const trimmed = input.trim();
  if (trimmed === "") return { ok: true, value: [] };
  if (/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(trimmed)) {
    try {
      return { ok: true, value: urlSearchParamsToParams(new URL(trimmed).searchParams) };
    } catch {
      return { ok: false, message: "无法解析该 URL" };
    }
  }
  const query = trimmed.startsWith("?") ? trimmed.slice(1) : trimmed;
  return { ok: true, value: urlSearchParamsToParams(new URLSearchParams(query)) };
};

const urlSearchParamsToParams = (searchParams: URLSearchParams): QueryParam[] =>
  [...searchParams.entries()].map(([key, value]) => ({ key, value }));

/** 键值表 → URL query 字符串（application/x-www-form-urlencoded，空格编码为 +）。恒成功。 */
export const paramsToQuery = (params: QueryParam[]): string => {
  const searchParams = new URLSearchParams();
  for (const { key, value } of params) {
    searchParams.append(key, value);
  }
  return searchParams.toString();
};
