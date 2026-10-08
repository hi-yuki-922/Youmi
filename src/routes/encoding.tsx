import { createFileRoute } from "@tanstack/react-router";
import { Plus, X } from "lucide-react";
import { useId, useState } from "react";

import { CopyButton } from "@/components/copy-button";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  base64ToText,
  dataUriToSvg,
  paramsToQuery,
  parseQuery,
  svgToDataUri,
  textToBase64,
  type ConvertResult,
  type QueryParam,
} from "@/tools/encoding/core";

export const Route = createFileRoute("/encoding")({
  component: EncodingToolPage,
});

const SUB_TOOLS = [
  { id: "svg-url", label: "SVG ↔ URL 编码" },
  { id: "base64", label: "文本 ↔ Base64" },
  { id: "query", label: "URL Query ↔ 键值表" },
] as const;

type SubToolId = (typeof SUB_TOOLS)[number]["id"];

function EncodingToolPage() {
  const [active, setActive] = useState<SubToolId>("svg-url");

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <h1 className="text-xl font-semibold">编码转换</h1>
      <div
        role="tablist"
        aria-label="编码转换子工具"
        className="flex w-fit gap-1 rounded-lg bg-muted p-1"
      >
        {SUB_TOOLS.map((tool) => (
          <button
            key={tool.id}
            type="button"
            role="tab"
            aria-selected={active === tool.id}
            onClick={() => setActive(tool.id)}
            className={cn(
              "rounded-md px-3 py-1.5 text-sm transition-colors",
              active === tool.id
                ? "bg-background font-medium text-foreground shadow-xs"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {tool.label}
          </button>
        ))}
      </div>

      {active === "svg-url" && (
        <InstantConverter
          hint="双向即时转换，编码结果可直接用于 CSS background-image。"
          leftLabel="SVG 源码"
          leftPlaceholder="粘贴 SVG 源码"
          rightLabel="URL 编码（data URI）"
          rightPlaceholder="粘贴 data URI 或编码后的文本"
          toRight={svgToDataUri}
          toLeft={dataUriToSvg}
        />
      )}
      {active === "base64" && (
        <InstantConverter
          hint="双向即时转换，按 UTF-8 编解码，兼容 base64url 与换行分隔的输入。"
          leftLabel="文本"
          leftPlaceholder="输入或粘贴文本"
          rightLabel="Base64"
          rightPlaceholder="粘贴 Base64 字符串"
          toRight={textToBase64}
          toLeft={base64ToText}
        />
      )}
      {active === "query" && <QueryTableTool />}
    </div>
  );
}

function ConverterPane({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}) {
  const id = useId();

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <label htmlFor={id} className="text-sm font-medium text-muted-foreground">
          {label}
        </label>
        <CopyButton text={value} variant="ghost" />
      </div>
      <Textarea
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="min-h-0 flex-1 resize-none font-mono text-sm"
        spellCheck={false}
      />
    </div>
  );
}

function ErrorStrip({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div className="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">{message}</div>
  );
}

/**
 * 双栏即时转换器：编辑任意一侧，另一侧立即更新。
 * 左 → 右恒成功；右 → 左可能失败，失败时保留左栏旧值并显示错误条。
 */
function InstantConverter({
  hint,
  leftLabel,
  leftPlaceholder,
  rightLabel,
  rightPlaceholder,
  toRight,
  toLeft,
}: {
  hint: string;
  leftLabel: string;
  leftPlaceholder: string;
  rightLabel: string;
  rightPlaceholder: string;
  toRight: (value: string) => string;
  toLeft: (value: string) => ConvertResult<string>;
}) {
  const [left, setLeft] = useState("");
  const [right, setRight] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleLeftChange = (value: string) => {
    setLeft(value);
    setRight(toRight(value));
    setError(null);
  };

  const handleRightChange = (value: string) => {
    setRight(value);
    const result = toLeft(value);
    if (result.ok) {
      setLeft(result.value);
      setError(null);
    } else {
      setError(result.message);
    }
  };

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-4">
      <p className="text-sm text-muted-foreground">{hint}</p>
      <ErrorStrip message={error} />
      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <ConverterPane
          label={leftLabel}
          value={left}
          onChange={handleLeftChange}
          placeholder={leftPlaceholder}
        />
        <ConverterPane
          label={rightLabel}
          value={right}
          onChange={handleRightChange}
          placeholder={rightPlaceholder}
        />
      </div>
    </section>
  );
}

type QueryRow = QueryParam & { id: string };

function QueryTableTool() {
  const [rows, setRows] = useState<QueryRow[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleQueryChange = (value: string) => {
    setQuery(value);
    const result = parseQuery(value);
    if (result.ok) {
      setRows(result.value.map((param) => ({ ...param, id: crypto.randomUUID() })));
      setError(null);
    } else {
      setError(result.message);
    }
  };

  const applyRows = (next: QueryRow[]) => {
    setRows(next);
    setQuery(paramsToQuery(next));
    setError(null);
  };

  const handleRowChange = (id: string, field: "key" | "value", value: string) => {
    applyRows(rows.map((row) => (row.id === id ? { ...row, [field]: value } : row)));
  };

  const handleAddRow = () => {
    applyRows([...rows, { id: crypto.randomUUID(), key: "", value: "" }]);
  };

  const handleRemoveRow = (id: string) => {
    applyRows(rows.filter((row) => row.id !== id));
  };

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-4">
      <p className="text-sm text-muted-foreground">双向即时转换：编辑任意一侧，另一侧同步更新。</p>
      <ErrorStrip message={error} />
      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <ConverterPane
          label="Query 字符串"
          value={query}
          onChange={handleQueryChange}
          placeholder="粘贴 query 字符串或完整 URL"
        />
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-muted-foreground">键值表</span>
            <Button variant="ghost" size="sm" onClick={handleAddRow}>
              <Plus data-slot="icon" />
              添加行
            </Button>
          </div>
          {rows.length === 0 ? (
            <div className="flex flex-1 items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
              编辑左侧 query，或添加一行
            </div>
          ) : (
            <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-1">
              {rows.map((row) => (
                <div key={row.id} className="flex items-center gap-2">
                  <Input
                    value={row.key}
                    placeholder="键"
                    onChange={(event) => handleRowChange(row.id, "key", event.target.value)}
                    className="font-mono text-sm"
                    spellCheck={false}
                    aria-label="键"
                  />
                  <Input
                    value={row.value}
                    placeholder="值"
                    onChange={(event) => handleRowChange(row.id, "value", event.target.value)}
                    className="font-mono text-sm"
                    spellCheck={false}
                    aria-label="值"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={`删除 ${row.key || "该行"}`}
                    onClick={() => handleRemoveRow(row.id)}
                  >
                    <X data-slot="icon" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
