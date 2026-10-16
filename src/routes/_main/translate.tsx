import { Link, createFileRoute } from "@tanstack/react-router";
import { ArrowLeftRight, Languages } from "lucide-react";
import { useEffect, useState } from "react";

import { ClearButton } from "@/components/ClearButton";
import { CopyButton } from "@/components/CopyButton";
import { Button, buttonVariants } from "@/components/ui/Button";
import { Kbd } from "@/components/ui/Kbd";
import { LabeledTextarea } from "@/components/ui/LabeledTextarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select";
import { hasTranslateApiKey, useTranslateStream } from "@/lib/translate";
import { restoreOneOf, restoreString, usePersistedToolState } from "@/lib/tool-state";
import { cn } from "@/lib/utils";
import {
  buildTranslatePairPrompt,
  isTranslatable,
  type TranslateLanguage,
} from "@/tools/translate/core";

/** 语言下拉选项：值即提示词层的语言参数。 */
const LANGUAGE_OPTIONS: ReadonlyArray<{ value: TranslateLanguage; label: string }> = [
  { value: "zh", label: "中文" },
  { value: "en", label: "英文" },
];

const restoreLanguage = restoreOneOf<TranslateLanguage>(
  LANGUAGE_OPTIONS.map((option) => option.value),
);

const LanguageSelect = ({
  label,
  value,
  onChange,
}: {
  label: string;
  value: TranslateLanguage;
  onChange: (value: TranslateLanguage) => void;
}) => (
  <Select value={value} onValueChange={(value) => onChange(value as TranslateLanguage)}>
    <SelectTrigger aria-label={label} className="w-28 shrink-0">
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      {LANGUAGE_OPTIONS.map((option) => (
        <SelectItem key={option.value} value={option.value}>
          {option.label}
        </SelectItem>
      ))}
    </SelectContent>
  </Select>
);

/** 译文区：与划词翻译浮层同款流式呈现——增长文本 + 流式中的脉冲光标。 */
const TranslationOutput = ({
  output,
  error,
  streaming,
}: {
  output: string;
  error: string | null;
  streaming: boolean;
}) => (
  <div className="min-h-0 flex-1 overflow-y-auto rounded-md border px-3 py-2 text-sm leading-relaxed break-words whitespace-pre-wrap">
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
      <span className="text-muted-foreground">译文将显示在这里</span>
    )}
  </div>
);

const TranslatePage = () => {
  const [source, setSource] = usePersistedToolState<TranslateLanguage>(
    "translate.source",
    "en",
    restoreLanguage,
  );
  const [target, setTarget] = usePersistedToolState<TranslateLanguage>(
    "translate.target",
    "zh",
    restoreLanguage,
  );
  const [input, setInput] = usePersistedToolState("translate.input", "", restoreString);
  // null = 探测中；探测失败不冒充"未配置"，引导态只在确认未配置时出现
  const [hasKey, setHasKey] = useState<boolean | null>(null);
  // 缺 key 不走默认报错文案：工作台以引导态呈现
  const { output, error, streaming, run, stop, reset } = useTranslateStream({
    onMissingKey: () => setHasKey(false),
  });

  useEffect(() => {
    let active = true;
    void hasTranslateApiKey()
      .then((exists) => {
        if (active) setHasKey(exists);
      })
      .catch((cause) => console.error("读取 DeepSeek API key 状态失败", cause));
    return () => {
      active = false;
    };
  }, []);

  const translate = () => {
    stop();
    if (!isTranslatable(input)) return;
    void run(buildTranslatePairPrompt(input, source, target));
  };

  // 双侧各只有中/英：一侧选成与对侧相同即整体对调，恒保持源 ≠ 目标
  const changeSource = (next: TranslateLanguage) => {
    setSource(next);
    if (next === target) setTarget(source);
  };

  const changeTarget = (next: TranslateLanguage) => {
    setTarget(next);
    if (next === source) setSource(target);
  };

  const swap = () => {
    setSource(target);
    setTarget(source);
  };

  // 清空即重新开始：进行中的请求一并中止，旧译文与错误态不再有意义
  const clearInput = () => {
    reset();
    setInput("");
  };

  if (hasKey === false) {
    return (
      <div className="flex h-full flex-col gap-4 p-6">
        <h1 className="text-xl font-semibold">翻译</h1>
        <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 rounded-md border border-dashed p-6 text-center">
          <Languages className="size-8 text-muted-foreground" />
          <p className="text-sm">尚未配置 DeepSeek API Key，配置后即可在此中英互译</p>
          <p className="text-xs text-muted-foreground">API Key 与划词翻译共用，保存在系统凭据库</p>
          <Link
            to="/settings"
            className={cn(buttonVariants({ variant: "outline", size: "sm" }), "shrink-0")}
          >
            前往「设置 → 划词翻译」配置
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-6">
      <h1 className="text-xl font-semibold">翻译</h1>

      <div className="flex flex-wrap items-center gap-2">
        <LanguageSelect label="源语言" value={source} onChange={changeSource} />
        <Button
          variant="ghost"
          size="icon"
          aria-label="交换语言方向"
          title="交换语言方向"
          onClick={swap}
          className="size-8 shrink-0 text-muted-foreground"
        >
          <ArrowLeftRight />
        </Button>
        <LanguageSelect label="目标语言" value={target} onChange={changeTarget} />
        <div className="min-w-0 flex-1" />
        <Button onClick={() => void translate()} disabled={!isTranslatable(input)}>
          翻译
        </Button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <LabeledTextarea
          label="原文"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={(event) => {
            if (event.ctrlKey && event.key === "Enter") {
              event.preventDefault();
              void translate();
            }
          }}
          placeholder="输入要翻译的文本"
          action={
            <>
              <span className="flex items-center gap-0.5" title="Ctrl+Enter 翻译">
                <Kbd>⌃</Kbd>
                <Kbd>⏎</Kbd>
              </span>
              <ClearButton label="清空原文" onClick={clearInput} disabled={input === ""} />
            </>
          }
        />
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-medium text-muted-foreground">译文</span>
            <CopyButton text={output} />
          </div>
          <TranslationOutput output={output} error={error} streaming={streaming} />
        </div>
      </div>
    </div>
  );
};

export const Route = createFileRoute("/_main/translate")({
  component: TranslatePage,
});
