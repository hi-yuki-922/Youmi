import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createFileRoute } from "@tanstack/react-router";
import { Languages, Pin, PinOff } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { SegmentedTabs } from "@/components/ui/SegmentedTabs";
import { setOverlayPinned } from "@/lib/overlay";
import { getTranslateApiKey, getTranslateModel, streamTranslate } from "@/lib/translate";
import { cn } from "@/lib/utils";
import {
  isTranslatable,
  TRANSLATE_DIRECTIONS,
  type TranslateDirection,
} from "@/tools/translate/core";

/** 取词结果事件载荷（Rust `SelectionEventPayload` kebab-case tag 序列化）。 */
type SelectionEventPayload = { kind: "text"; text: string } | { kind: "message"; message: string };

const SELECTION_EVENT = "translate-selection";

const TranslateOverlay = () => {
  const [direction, setDirection] = useState<TranslateDirection>("auto-to-zh");
  const [source, setSource] = useState("");
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [pinned, setPinned] = useState(false);

  const abortRef = useRef<AbortController | null>(null);
  // 事件订阅只挂一次：经 ref 取当前方向，新取词始终按最新方向翻译
  const directionRef = useRef(direction);

  const stopStream = () => {
    abortRef.current?.abort();
    abortRef.current = null;
  };

  const translate = async (text: string, to: TranslateDirection) => {
    stopStream();
    if (!isTranslatable(text)) return;
    const controller = new AbortController();
    abortRef.current = controller;
    setOutput("");
    setError(null);
    setStreaming(true);
    try {
      const apiKey = await getTranslateApiKey();
      if (!apiKey) {
        setError("未配置 DeepSeek API Key，请在主窗口「设置 → 划词翻译」填写");
        return;
      }
      const model = await getTranslateModel();
      await streamTranslate({
        apiKey,
        model,
        text,
        direction: to,
        signal: controller.signal,
        onDelta: (delta) => setOutput((prev) => prev + delta),
      });
    } catch (cause) {
      // 换方向/新取词触发的中止不是失败
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

  // 浮层 hide 而非销毁：每次取词事件都是一轮新翻译；Esc 隐藏窗口。
  // 订阅只挂一次，translate 经 directionRef 读取最新方向，不进依赖。
  useEffect(() => {
    const unlisten = listen<SelectionEventPayload>(SELECTION_EVENT, (event) => {
      stopStream();
      setStreaming(false);
      if (event.payload.kind === "text") {
        setSource(event.payload.text);
        void translate(event.payload.text, directionRef.current);
      } else {
        setSource("");
        setOutput("");
        setError(event.payload.message);
      }
    });
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") void getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      stopStream();
      window.removeEventListener("keydown", onKeyDown);
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  const changeDirection = (next: TranslateDirection) => {
    setDirection(next);
    directionRef.current = next;
    if (isTranslatable(source)) void translate(source, next);
  };

  const togglePinned = () => {
    const next = !pinned;
    setPinned(next);
    setOverlayPinned(next).catch((cause) => {
      console.error("切换固定浮层失败", cause);
      setPinned(!next);
    });
  };

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-sm">
      <div className="flex items-center gap-2 border-b px-3 py-2">
        <SegmentedTabs
          aria-label="翻译方向"
          options={TRANSLATE_DIRECTIONS}
          value={direction}
          onChange={changeDirection}
          className="shrink-0"
        />
        <div className="min-w-0 flex-1" />
        <button
          type="button"
          onClick={togglePinned}
          aria-label={pinned ? "取消固定浮层" : "固定浮层"}
          aria-pressed={pinned}
          title={pinned ? "取消固定：失焦自动隐藏" : "固定：失焦不再自动隐藏"}
          className={cn(
            "shrink-0 rounded p-1.5 transition-colors",
            pinned
              ? "bg-accent text-foreground"
              : "text-muted-foreground hover:bg-accent hover:text-foreground",
          )}
        >
          {pinned ? <PinOff className="size-4" /> : <Pin className="size-4" />}
        </button>
      </div>

      {source && (
        <p className="line-clamp-3 break-all whitespace-pre-wrap border-b px-4 py-2 text-xs leading-relaxed text-muted-foreground">
          {source}
        </p>
      )}

      <div className="flex-1 overflow-y-auto px-4 py-3 leading-relaxed whitespace-pre-wrap break-words">
        {error ? (
          <p className="text-destructive">{error}</p>
        ) : output ? (
          <>
            {output}
            {streaming && <span className="animate-pulse">▍</span>}
          </>
        ) : streaming ? (
          <span className="animate-pulse">▍</span>
        ) : (
          <p className="flex items-center gap-2 text-muted-foreground">
            <Languages className="size-4" />
            选中文字后按 Alt+T 翻译
          </p>
        )}
      </div>

      <p className="border-t px-4 py-1.5 text-xs text-muted-foreground">
        切换方向重新翻译 · Esc 关闭 · 图钉固定后失焦不隐藏
      </p>
    </div>
  );
};

export const Route = createFileRoute("/overlay/translate")({
  component: TranslateOverlay,
});
