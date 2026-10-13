/**
 * 划词翻译的 tauri 边界：DeepSeek 配置与流式请求。
 * API key 存系统凭据库（Rust command）；模型名存设置文件（非秘密）。
 */

import { createOpenAICompatible } from "@ai-sdk/openai-compatible";
import { invoke } from "@tauri-apps/api/core";
import { streamText } from "ai";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";

import { getSetting, setSetting } from "@/lib/settings";
import { buildTranslatePrompt, type TranslateDirection } from "@/tools/translate/core";

const DEEPSEEK_BASE_URL = "https://api.deepseek.com/v1";
const MODEL_KEY = "translate.model";

export const DEEPSEEK_DEFAULT_MODEL = "deepseek-chat";

export const getTranslateModel = (): Promise<string> =>
  getSetting(MODEL_KEY, DEEPSEEK_DEFAULT_MODEL);

export const setTranslateModel = (model: string): Promise<void> => setSetting(MODEL_KEY, model);

/** 凭据库中的 DeepSeek API key；未配置返回 null。 */
export const getTranslateApiKey = (): Promise<string | null> =>
  invoke<string | null>("translate_get_api_key");

/** 保存 DeepSeek API key 到系统凭据库；空白值 = 清除。 */
export const setTranslateApiKey = (key: string): Promise<void> =>
  invoke("translate_set_api_key", { key });

export interface StreamTranslateParams {
  apiKey: string;
  model: string;
  text: string;
  direction: TranslateDirection;
  signal?: AbortSignal;
  onDelta: (delta: string) => void;
}

/**
 * 流式翻译：请求经 tauri http 插件（绕开 webview CORS）发给 DeepSeek 的
 * OpenAI-compatible chat 接口，译文逐段回调。
 */
export const streamTranslate = async ({
  apiKey,
  model,
  text,
  direction,
  signal,
  onDelta,
}: StreamTranslateParams): Promise<void> => {
  const provider = createOpenAICompatible({
    name: "deepseek",
    baseURL: DEEPSEEK_BASE_URL,
    apiKey,
    fetch: tauriFetch,
  });
  const { system, prompt } = buildTranslatePrompt(text, direction);

  // AI SDK 的 textStream 不递出错误：错误经 onError 汇集，流结束后统一抛出
  let failure: unknown;
  const result = streamText({
    model: provider.chatModel(model),
    system,
    prompt,
    abortSignal: signal,
    onError: (error) => {
      failure = error;
    },
  });
  for await (const delta of result.textStream) {
    onDelta(delta);
  }
  if (failure !== undefined) {
    throw failure instanceof Error ? failure : new Error(String(failure));
  }
};
