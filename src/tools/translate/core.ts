/** 翻译的纯逻辑：方向/语言对清单与提示词构造，无 tauri/网络依赖。 */

/** 划词翻译方向。auto 检测源语言：源文已是中文时改译英文（避免中→中空转）。 */
export type TranslateDirection = "auto-to-zh" | "zh-to-en" | "en-to-zh";

export const TRANSLATE_DIRECTIONS: ReadonlyArray<{
  id: TranslateDirection;
  label: string;
}> = [
  { id: "auto-to-zh", label: "自动 → 中文" },
  { id: "zh-to-en", label: "中文 → 英文" },
  { id: "en-to-zh", label: "英文 → 中文" },
];

/** 翻译语言：提示词层的泛化方向参数，源/目标各取其一（无自动检测）。 */
export type TranslateLanguage = "zh" | "en";

/** 发给模型的翻译请求提示词（OpenAI-compatible chat 的 system + user 消息）。 */
export interface TranslatePrompt {
  system: string;
  prompt: string;
}

const OUTPUT_RULE =
  "Output ONLY the translation itself: no explanations, no surrounding quotes, no leading or trailing whitespace. Preserve line breaks and keep inline code, numbers and URLs as-is.";

const AUTO_RULE =
  "Detect the source language automatically. If the text is already in Chinese, translate it into English; otherwise translate it into Simplified Chinese.";

// 源语言与目标语言分表：中文作源用泛称（Chinese），作目标明确为简体（Simplified Chinese）
const SOURCE_NAMES: Record<TranslateLanguage, string> = {
  zh: "Chinese",
  en: "English",
};

const TARGET_NAMES: Record<TranslateLanguage, string> = {
  zh: "Simplified Chinese",
  en: "English",
};

/** 固定语言对规则：源/目标语言均已知。划词翻译的固定方向与工作台双下拉共用。 */
const languagePairRule = (source: TranslateLanguage, target: TranslateLanguage): string =>
  `Translate the text from ${SOURCE_NAMES[source]} into ${TARGET_NAMES[target]}.`;

const DIRECTION_RULES: Record<TranslateDirection, string> = {
  "auto-to-zh": AUTO_RULE,
  "zh-to-en": languagePairRule("zh", "en"),
  "en-to-zh": languagePairRule("en", "zh"),
};

const buildPrompt = (text: string, rule: string): TranslatePrompt => ({
  system: `You are a translation engine. ${rule} ${OUTPUT_RULE}`,
  prompt: text,
});

/** 划词翻译方向提示词（auto 规则 + 两个固定语言对）。 */
export const buildTranslatePrompt = (
  text: string,
  direction: TranslateDirection,
): TranslatePrompt => buildPrompt(text, DIRECTION_RULES[direction]);

/** 工作台语言对提示词：与划词翻译的固定方向走同一规则层。 */
export const buildTranslatePairPrompt = (
  text: string,
  source: TranslateLanguage,
  target: TranslateLanguage,
): TranslatePrompt => buildPrompt(text, languagePairRule(source, target));

export const isTranslatable = (text: string): boolean => text.trim().length > 0;
