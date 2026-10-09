import { parseDiffFromFile, type FileContents } from "@pierre/diffs";
import { FileDiff } from "@pierre/diffs/react";
import { createFileRoute } from "@tanstack/react-router";
import { useId, useMemo, useState } from "react";

import { LabeledTextarea } from "@/components/ui/labeled-textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SegmentedTabs } from "@/components/ui/segmented-tabs";

export const Route = createFileRoute("/diff")({
  component: DiffToolPage,
});

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
function toFileContents(name: string, contents: string, lang: LanguageId): FileContents {
  return lang === "text" ? { name, contents } : { name, contents, lang };
}

function DiffToolPage() {
  const [view, setView] = useState<ViewId>("split");
  const [lang, setLang] = useState<LanguageId>("text");
  const [oldText, setOldText] = useState("");
  const [newText, setNewText] = useState("");
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
        />
        <LabeledTextarea
          label="新文本"
          value={newText}
          onChange={(event) => setNewText(event.target.value)}
          placeholder="粘贴新文本"
        />
      </div>

      {bothEmpty ? (
        <div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
          在上方粘贴两段文本，即时对比
        </div>
      ) : (
        <FileDiff
          fileDiff={fileDiff}
          className="min-h-0 flex-1 overflow-hidden rounded-md border"
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
}
