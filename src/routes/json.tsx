import { createFileRoute } from "@tanstack/react-router";
import { Check, Copy } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import {
  escapeToJsonString,
  formatJson,
  minifyJson,
  unescapeFromJsonString,
  validateJson,
  type JsonFailure,
  type JsonIndent,
} from "@/tools/json/core";

export const Route = createFileRoute("/json")({
  component: JsonToolPage,
});

type Status =
  | { kind: "idle" }
  | { kind: "success"; text: string }
  | { kind: "error"; text: string; line?: number; column?: number };

function failureToStatus(failure: JsonFailure): Status {
  return { kind: "error", text: failure.message, line: failure.line, column: failure.column };
}

function JsonToolPage() {
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [indent, setIndent] = useState<JsonIndent>(2);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [copied, setCopied] = useState(false);

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

  const handleCopy = async () => {
    if (!output) return;
    await navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
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
          <Button variant="outline" size="sm" onClick={handleCopy} disabled={!output}>
            {copied ? <Check data-slot="icon" /> : <Copy data-slot="icon" />}
            {copied ? "已复制" : "复制结果"}
          </Button>
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
        <div
          className={
            status.kind === "success"
              ? "rounded-md bg-emerald-500/10 px-3 py-2 text-sm text-emerald-600 dark:text-emerald-400"
              : "rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive"
          }
        >
          {status.kind === "error" && status.line !== undefined && status.column !== undefined
            ? `第 ${status.line} 行第 ${status.column} 列：${status.text}`
            : status.text}
        </div>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <div className="flex min-h-0 flex-col gap-2">
          <label htmlFor="json-input" className="text-sm font-medium text-muted-foreground">
            输入
          </label>
          <Textarea
            id="json-input"
            value={input}
            onChange={(event) => setInput(event.target.value)}
            placeholder="粘贴或输入 JSON 文本"
            className="min-h-0 flex-1 resize-none font-mono text-sm"
            spellCheck={false}
          />
        </div>
        <div className="flex min-h-0 flex-col gap-2">
          <label htmlFor="json-output" className="text-sm font-medium text-muted-foreground">
            输出
          </label>
          <Textarea
            id="json-output"
            value={output}
            readOnly
            placeholder="结果将显示在这里"
            className="min-h-0 flex-1 resize-none font-mono text-sm"
            spellCheck={false}
          />
        </div>
      </div>
    </div>
  );
}
