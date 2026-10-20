import { createFileRoute } from "@tanstack/react-router";

import { ClearButton } from "@/components/ClearButton";
import { CopyButton } from "@/components/CopyButton";
import { LabeledCodeEditor } from "@/components/CodeEditor";
import { Button } from "@/components/ui/Button";
import { FeedbackStrip } from "@/components/ui/FeedbackStrip";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select";
import { restoreString, usePersistedToolState } from "@/lib/tool-state";
import {
  escapeToJsonString,
  formatJson,
  minifyJson,
  unescapeFromJsonString,
  validateJson,
  type JsonFailure,
  type JsonIndent,
} from "@/tools/json/core";

type Status =
  | { kind: "idle" }
  | { kind: "success"; text: string }
  | { kind: "error"; text: string; line?: number; column?: number };

const failureToStatus = (failure: JsonFailure): Status => ({
  kind: "error",
  text: failure.message,
  line: failure.line,
  column: failure.column,
});

const restoreIndent = (value: unknown): JsonIndent | undefined =>
  value === 2 || value === 4 || value === "tab" ? value : undefined;

const restoreStatus = (value: unknown): Status | undefined => {
  if (typeof value !== "object" || value === null) return undefined;
  const record = value as { kind?: unknown; text?: unknown; line?: unknown; column?: unknown };
  if (record.kind === "idle") return { kind: "idle" };
  if (record.kind !== "success" && record.kind !== "error") return undefined;
  if (typeof record.text !== "string") return undefined;
  const line = typeof record.line === "number" ? record.line : undefined;
  const column = typeof record.column === "number" ? record.column : undefined;
  return record.kind === "success"
    ? { kind: "success", text: record.text }
    : { kind: "error", text: record.text, line, column };
};

const JsonToolPage = () => {
  const [input, setInput] = usePersistedToolState("json.input", "", restoreString);
  const [output, setOutput] = usePersistedToolState("json.output", "", restoreString);
  const [indent, setIndent] = usePersistedToolState<JsonIndent>("json.indent", 2, restoreIndent);
  const [status, setStatus] = usePersistedToolState<Status>(
    "json.status",
    { kind: "idle" },
    restoreStatus,
  );

  const runTransform = (
    transform: () => { ok: true; value: string } | JsonFailure,
    successText: string,
  ) => {
    const result = transform();
    if (result.ok) {
      setOutput(result.value);
      setStatus({ kind: "success", text: successText });
    } else {
      setStatus(failureToStatus(result));
    }
  };

  const handleFormat = () => runTransform(() => formatJson(input, indent), "格式化完成");
  const handleMinify = () => runTransform(() => minifyJson(input), "压缩完成");
  const handleEscape = () =>
    runTransform(() => ({ ok: true, value: escapeToJsonString(input) }), "转义完成");
  const handleUnescape = () => runTransform(() => unescapeFromJsonString(input), "去转义完成");

  const handleValidate = () => {
    const result = validateJson(input);
    setStatus(result.ok ? { kind: "success", text: "JSON 合法" } : failureToStatus(result));
  };

  // 输入清空联动输出与状态条：旧输入的结果不再有意义
  const clearInput = () => {
    setInput("");
    setOutput("");
    setStatus({ kind: "idle" });
  };

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">JSON 工具</h1>
        <div className="flex items-center gap-3">
          <span className="text-sm text-muted-foreground">缩进</span>
          <Select
            value={String(indent)}
            onValueChange={(value) => setIndent(value === "2" ? 2 : value === "4" ? 4 : "tab")}
          >
            <SelectTrigger className="w-28">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="2">2 空格</SelectItem>
              <SelectItem value="4">4 空格</SelectItem>
              <SelectItem value="tab">Tab</SelectItem>
            </SelectContent>
          </Select>
          <CopyButton text={output} />
        </div>
      </div>

      <div className="flex items-center gap-2">
        <Button onClick={handleFormat}>格式化</Button>
        <Button variant="secondary" onClick={handleMinify}>
          压缩
        </Button>
        <Button variant="secondary" onClick={handleValidate}>
          校验
        </Button>
        <Button variant="secondary" onClick={handleEscape}>
          转义
        </Button>
        <Button variant="secondary" onClick={handleUnescape}>
          去转义
        </Button>
      </div>

      {status.kind !== "idle" && (
        <FeedbackStrip kind={status.kind}>
          {status.kind === "error" && status.line !== undefined && status.column !== undefined
            ? `第 ${status.line} 行第 ${status.column} 列：${status.text}`
            : status.text}
        </FeedbackStrip>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <LabeledCodeEditor
          label="输入"
          ariaLabel="JSON 输入"
          value={input}
          onChange={setInput}
          placeholder="粘贴或输入 JSON 文本"
          action={<ClearButton label="清空输入" onClick={clearInput} disabled={input === ""} />}
        />
        <LabeledCodeEditor
          label="输出"
          ariaLabel="JSON 输出"
          value={output}
          readOnly
          placeholder="结果将显示在这里"
        />
      </div>
    </div>
  );
};

export const Route = createFileRoute("/_main/json")({
  component: JsonToolPage,
});
