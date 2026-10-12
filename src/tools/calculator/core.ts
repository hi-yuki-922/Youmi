export type EvaluateResult = { ok: true; value: number } | { ok: false; message: string };

class ExpressionError extends Error {}

type Token =
  | { kind: "number"; value: number }
  | { kind: "op"; char: "+" | "-" | "*" | "/" }
  | { kind: "paren"; char: "(" | ")" };

/** 输入归一化：中文键盘常见的 × ÷ 与全角括号按等价半角符号处理。 */
const normalize = (input: string): string =>
  input.replace(/×/g, "*").replace(/÷/g, "/").replace(/（/g, "(").replace(/）/g, ")");

const tokenize = (input: string): Token[] => {
  const tokens: Token[] = [];
  let i = 0;
  while (i < input.length) {
    const ch = input[i] as string;
    if (ch === " ") {
      i++;
    } else if ("+-*/".includes(ch)) {
      tokens.push({ kind: "op", char: ch as "+" | "-" | "*" | "/" });
      i++;
    } else if (ch === "(" || ch === ")") {
      tokens.push({ kind: "paren", char: ch });
      i++;
    } else if (/[0-9.]/.test(ch)) {
      const matched = /^(\d+(\.\d+)?|\.\d+)/.exec(input.slice(i))?.[0];
      if (!matched) throw new ExpressionError("算式不完整");
      tokens.push({ kind: "number", value: Number(matched) });
      i += matched.length;
    } else {
      throw new ExpressionError(`存在无法识别的字符「${ch}」`);
    }
  }
  return tokens;
};

/** 光标位置用可变对象承载，递归下降各层共享推进。 */
type Cursor = { index: number };

const currentToken = (tokens: Token[], cursor: Cursor): Token | undefined => tokens[cursor.index];

/** expr := term (('+' | '-') term)* */
const parseExpression = (tokens: Token[], cursor: Cursor): number => {
  let left = parseTerm(tokens, cursor);
  for (;;) {
    const token = currentToken(tokens, cursor);
    if (token?.kind !== "op" || (token.char !== "+" && token.char !== "-")) break;
    cursor.index++;
    const right = parseTerm(tokens, cursor);
    left = token.char === "+" ? left + right : left - right;
  }
  return left;
};

/** term := factor (('*' | '/') factor)* */
const parseTerm = (tokens: Token[], cursor: Cursor): number => {
  let left = parseFactor(tokens, cursor);
  for (;;) {
    const token = currentToken(tokens, cursor);
    if (token?.kind !== "op" || (token.char !== "*" && token.char !== "/")) break;
    cursor.index++;
    const right = parseFactor(tokens, cursor);
    if (token.char === "/") {
      if (right === 0) throw new ExpressionError("除数不能为零");
      left /= right;
    } else {
      left *= right;
    }
  }
  return left;
};

/** factor := ('+' | '-')? primary —— 一元符号至多一个，"1++2" 这类连打按误输入拒绝。 */
const parseFactor = (tokens: Token[], cursor: Cursor): number => {
  const token = currentToken(tokens, cursor);
  if (token?.kind === "op" && (token.char === "+" || token.char === "-")) {
    cursor.index++;
    return token.char === "-" ? -parsePrimary(tokens, cursor) : parsePrimary(tokens, cursor);
  }
  return parsePrimary(tokens, cursor);
};

/** primary := number | '(' expr ')' */
const parsePrimary = (tokens: Token[], cursor: Cursor): number => {
  const token = currentToken(tokens, cursor);
  if (token?.kind === "number") {
    cursor.index++;
    return token.value;
  }
  if (token?.kind === "paren" && token.char === "(") {
    cursor.index++;
    const value = parseExpression(tokens, cursor);
    const closing = currentToken(tokens, cursor);
    if (closing?.kind !== "paren" || closing.char !== ")") {
      throw new ExpressionError("括号不配对");
    }
    cursor.index++;
    return value;
  }
  throw new ExpressionError("算式不完整");
};

/**
 * 四则运算求值：+ - * /、括号、一元正负号、小数，空格随意。
 * 非法输入（不完整、不配对、无法识别字符、除零、溢出）返回失败并带原因。
 */
export const evaluate = (input: string): EvaluateResult => {
  try {
    const tokens = tokenize(normalize(input));
    const cursor: Cursor = { index: 0 };
    const value = parseExpression(tokens, cursor);
    if (cursor.index < tokens.length) throw new ExpressionError("算式无法完整解析");
    if (!Number.isFinite(value)) throw new ExpressionError("结果超出可表示范围");
    return { ok: true, value };
  } catch (error) {
    if (error instanceof ExpressionError) return { ok: false, message: error.message };
    throw error;
  }
};

/** 是否值得按算式求值：需同时含数字与运算符——文本查询常含连字符，不能误报。 */
export const isCalculatable = (input: string): boolean =>
  /\d/.test(input) && /[+\-*/×÷]/.test(input);

/** 结果显示：12 位有效数字内取整，消去浮点噪声（0.1+0.2 → 0.3）。 */
export const formatResult = (value: number): string => String(Number(value.toPrecision(12)));
