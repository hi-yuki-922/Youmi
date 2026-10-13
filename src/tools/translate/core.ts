/** 划词翻译的纯逻辑：方向清单与提示词构造，无 tauri/网络依赖。 */

/** 翻译方向。auto 检测源语言：源文已是中文时改译英文（避免中→中空转）。 */
export type TranslateDirection = "auto-to-zh" | "zh-to-en" | "en-to-zh";

export const TRANSLATE_DIRECTIONS: ReadonlyArray<{
  id: TranslateDirection;
  label: string;
}> = [
  { id: "auto-to-zh", label: "自动 → 中文" },
  { id: "zh-to-en", label: "中文 → 英文" },
  { id: "en-to-zh", label: "英文 → 中文" },
];

/** 发给模型的翻译请求提示词（OpenAI-compatible chat 的 system + user 消息）。 */
export interface TranslatePrompt {
  system: string;
  prompt: string;
}

const OUTPUT_RULE =
  "Output ONLY the translation itself: no explanations, no surrounding quotes, no leading or trailing whitespace. Preserve line breaks and keep inline code, numbers and URLs as-is.";

const AUTO_RULE =
  "Detect the source language automatically. If the text is already in Chinese, translate it into English; otherwise translate it into Simplified Chinese.";

const DIRECTION_RULES: Record<TranslateDirection, string> = {
  "auto-to-zh": AUTO_RULE,
  "zh-to-en": "Translate the text from Chinese into English.",
  "en-to-zh": "Translate the text from English into Simplified Chinese.",
};

export const buildTranslatePrompt = (
  text: string,
  direction: TranslateDirection,
): TranslatePrompt => ({
  system: `You are a translation engine. ${DIRECTION_RULES[direction]} ${OUTPUT_RULE}`,
  prompt: text,
});

export const isTranslatable = (text: string): boolean => text.trim().length > 0;
