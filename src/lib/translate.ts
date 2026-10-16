/**
 * DeepSeek 流式翻译的 tauri 边界（划词翻译浮层与主窗口翻译页共用）。
 * API key 存系统凭据库（Rust command）；模型名存设置文件（非秘密）。
 * 提示词由 `tools/translate/core` 构造后传入，本模块只管传输。
 */

import { createOpenAICompatible } from "@ai-sdk/openai-compatible";
import { invoke } from "@tauri-apps/api/core";
import { streamText } from "ai";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { useEffect, useRef, useState } from "react";

import { getSetting, setSetting } from "@/lib/settings";
import type { TranslatePrompt } from "@/tools/translate/core";

const DEEPSEEK_BASE_URL = "https://api.deepseek.com/v1";
const MODEL_KEY = "translate.model";

export const DEEPSEEK_DEFAULT_MODEL = "deepseek-flash";

export const getTranslateModel = (): Promise<string> =>
  getSetting(MODEL_KEY, DEEPSEEK_DEFAULT_MODEL);

export const setTranslateModel = (model: string): Promise<void> => setSetting(MODEL_KEY, model);

/** 凭据库中是否已配置 API key（presence 查询，不取回明文）。 */
export const hasTranslateApiKey = (): Promise<boolean> => invoke<boolean>("translate_has_api_key");

/** 凭据库中的 DeepSeek API key；未配置返回 null。仅翻译发起时取回。 */
export const getTranslateApiKey = (): Promise<string | null> =>
  invoke<string | null>("translate_get_api_key");

/** 保存 DeepSeek API key 到系统凭据库；空白值 = 清除。 */
export const setTranslateApiKey = (key: string): Promise<void> =>
  invoke("translate_set_api_key", { key });

export interface StreamTranslateParams {
  apiKey: string;
  model: string;
  prompt: TranslatePrompt;
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
  prompt,
  signal,
  onDelta,
}: StreamTranslateParams): Promise<void> => {
  const provider = createOpenAICompatible({
    name: "deepseek",
    baseURL: DEEPSEEK_BASE_URL,
    apiKey,
    fetch: tauriFetch,
  });

  // AI SDK 的 textStream 不递出错误：错误经 onError 汇集，流结束后统一抛出
  let failure: unknown;
  const result = streamText({
    model: provider.chatModel(model),
    system: prompt.system,
    prompt: prompt.prompt,
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

/**
 * 流式翻译会话状态：中止控制器生命周期、发起时现取 key/模型、delta 追加；
 * 翻译中再触发即中止上一次（主动中止不算失败），卸载时自动中止。
 * 划词翻译浮层与主窗口翻译页共用；缺 key 的处置（浮层报错 / 工作台引导态）
 * 经 onMissingKey 交给调用方，未提供时回退浮层的报错文案。
 */
export const useTranslateStream = (options?: { onMissingKey?: () => void }) => {
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [streaming, setStreaming] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const stopStream = () => {
    abortRef.current?.abort();
    abortRef.current = null;
  };

  // 路由切走/浮层销毁即中止：流回调不再触碰已卸载的页面状态
  useEffect(() => () => stopStream(), []);

  /** 中止当前流并清空译文与错误态，下一次翻译从干净现场开始。 */
  const reset = () => {
    stopStream();
    setOutput("");
    setError(null);
    setStreaming(false);
  };

  /** 流式翻译一条提示词；发起即取代上一次进行中的请求。 */
  const run = async (prompt: TranslatePrompt) => {
    stopStream();
    const controller = new AbortController();
    abortRef.current = controller;
    setOutput("");
    setError(null);
    setStreaming(true);
    try {
      // 每次发起都现取凭据与模型名：设置页改一处，下一发即生效
      const apiKey = await getTranslateApiKey();
      if (!apiKey) {
        if (options?.onMissingKey) options.onMissingKey();
        else setError("未配置 DeepSeek API Key，请在主窗口「设置 → 划词翻译」填写");
        return;
      }
      const model = await getTranslateModel();
      await streamTranslate({
        apiKey,
        model,
        prompt,
        signal: controller.signal,
        onDelta: (delta) => setOutput((prev) => prev + delta),
      });
    } catch (cause) {
      // 换方向/再触发导致的中止不是失败
      if (!controller.signal.aborted) {
        console.error("翻译请求失败", cause);
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    } finally {
      if (abortRef.current === controller) {
        abortRef.current = null;
        setStreaming(false);
      }
    }
  };

  return { output, setOutput, error, setError, streaming, run, stop: stopStream, reset };
};
