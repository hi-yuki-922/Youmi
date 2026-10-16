import { parseDiffFromFile, type FileContents } from "@pierre/diffs";
import { FileDiff } from "@pierre/diffs/react";
import { createFileRoute } from "@tanstack/react-router";
import { useId, useMemo } from "react";

import { ClearButton } from "@/components/ClearButton";
import { LabeledTextarea } from "@/components/ui/LabeledTextarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select";
import { SegmentedTabs } from "@/components/ui/SegmentedTabs";
import { restoreOneOf, restoreString, usePersistedToolState } from "@/lib/tool-state";

const VIEW_OPTIONS = [
  { id: "split", label: "并排" },
  { id: "unified", label: "统一" },
] as const;

type ViewId = (typeof VIEW_OPTIONS)[number]["id"];

const LANGUAGES = [
  { value: "text", label: "纯文本" },
  { value: "json", label: "JSON" },
  { value: "typescript", label: "TypeScript" },
  { value: "javascript", label: "JavaScript" },
  { value: "tsx", label: "TSX" },
  { value: "html", label: "HTML" },
  { value: "css", label: "CSS" },
  { value: "python", label: "Python" },
  { value: "rust", label: "Rust" },
  { value: "go", label: "Go" },
  { value: "java", label: "Java" },
  { value: "cpp", label: "C++" },
  { value: "csharp", label: "C#" },
  { value: "sql", label: "SQL" },
  { value: "yaml", label: "YAML" },
  { value: "markdown", label: "Markdown" },
  { value: "shellscript", label: "Shell" },
] as const;

type LanguageId = (typeof LANGUAGES)[number]["value"];

/** 纯文本不指定 lang，交给库按文件名推断；其余语言显式指定。 */
const toFileContents = (name: string, contents: string, lang: LanguageId): FileContents =>
  lang === "text" ? { name, contents } : { name, contents, lang };

const restoreView = restoreOneOf(VIEW_OPTIONS.map((option) => option.id));

const restoreLang = restoreOneOf(LANGUAGES.map((language) => language.value));

const DiffToolPage = () => {
  const [view, setView] = usePersistedToolState<ViewId>("diff.view", "split", restoreView);
  const [lang, setLang] = usePersistedToolState<LanguageId>("diff.lang", "text", restoreLang);
  const [oldText, setOldText] = usePersistedToolState("diff.old", "", restoreString);
  const [newText, setNewText] = usePersistedToolState("diff.new", "", restoreString);
  const langSelectId = useId();

  const fileDiff = useMemo(
    () =>
      parseDiffFromFile(
        toFileContents("original.txt", oldText, lang),
        toFileContents("modified.txt", newText, lang),
      ),
    [oldText, newText, lang],
  );

  const bothEmpty = oldText === "" && newText === "";

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-6">
      <h1 className="text-xl font-semibold">文本 diff</h1>
      <p className="text-sm text-muted-foreground">
        粘贴两段文本即时对比；语法高亮可在右上角选择语言。
      </p>

      <div className="flex flex-wrap items-center justify-between gap-2">
        <SegmentedTabs
          aria-label="diff 视图"
          options={VIEW_OPTIONS}
          value={view}
          onChange={setView}
        />
        <div className="flex items-center gap-2">
          <label htmlFor={langSelectId} className="text-sm text-muted-foreground">
            语言
          </label>
          <Select value={lang} onValueChange={(value) => setLang(value as LanguageId)}>
            <SelectTrigger id={langSelectId} className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LANGUAGES.map((item) => (
                <SelectItem key={item.value} value={item.value}>
                  {item.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <div className="grid h-40 grid-cols-2 gap-4">
        <LabeledTextarea
          label="原文本"
          value={oldText}
          onChange={(event) => setOldText(event.target.value)}
          placeholder="粘贴原文本"
          action={
            <ClearButton
              label="清空原文本"
              onClick={() => setOldText("")}
              disabled={oldText === ""}
            />
          }
        />
        <LabeledTextarea
          label="新文本"
          value={newText}
          onChange={(event) => setNewText(event.target.value)}
          placeholder="粘贴新文本"
          action={
            <ClearButton
              label="清空新文本"
              onClick={() => setNewText("")}
              disabled={newText === ""}
            />
          }
        />
      </div>

      {bothEmpty ? (
        <div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
          在上方粘贴两段文本，即时对比
        </div>
      ) : (
        <FileDiff
          fileDiff={fileDiff}
          // 滚动归 diff 库的内部 scroller：diffs-container 根元素即其垂直
          // 滚动容器（库在其上监听 scroll 驱动虚拟化），这里只开纵向滚动 +
          // 高度约束；overflow-hidden 会把库的滚动裁掉导致长内容不可滚
          className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto rounded-md border"
          options={{
            diffStyle: view,
            lineDiffType: "char",
            diffIndicators: "bars",
            overflow: "wrap",
            disableFileHeader: true,
          }}
        />
      )}
    </div>
  );
};

export const Route = createFileRoute("/_main/diff")({
  component: DiffToolPage,
});
