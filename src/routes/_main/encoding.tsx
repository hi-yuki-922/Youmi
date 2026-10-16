import { createFileRoute } from "@tanstack/react-router";
import { Plus, X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import { ClearButton } from "@/components/ClearButton";
import { CopyButton } from "@/components/CopyButton";
import { Button } from "@/components/ui/Button";
import { FeedbackStrip } from "@/components/ui/FeedbackStrip";
import { Input } from "@/components/ui/Input";
import { LabeledTextarea } from "@/components/ui/LabeledTextarea";
import { SegmentedTabs } from "@/components/ui/SegmentedTabs";
import {
  restoreNullableString,
  restoreOneOf,
  restoreString,
  usePersistedToolState,
} from "@/lib/tool-state";
import {
  base64ToText,
  dataUriToSvg,
  paramsToQuery,
  parseQuery,
  svgIconScss,
  svgToDataUri,
  textToBase64,
  type ConvertResult,
  type QueryParam,
} from "@/tools/encoding/core";

const SUB_TOOLS = [
  { id: "svg-url", label: "SVG ↔ URL 编码" },
  { id: "base64", label: "文本 ↔ Base64" },
  { id: "query", label: "URL Query ↔ 键值表" },
] as const;

type SubToolId = (typeof SUB_TOOLS)[number]["id"];

const restoreSubToolId = restoreOneOf(SUB_TOOLS.map((subTool) => subTool.id));

const EncodingToolPage = () => {
  const [active, setActive] = usePersistedToolState<SubToolId>(
    "encoding.tab",
    "svg-url",
    restoreSubToolId,
  );

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <h1 className="text-xl font-semibold">编码转换</h1>
      <SegmentedTabs
        aria-label="编码转换子工具"
        options={SUB_TOOLS}
        value={active}
        onChange={setActive}
      />

      {active === "svg-url" && (
        <InstantConverter
          stateKey="encoding.svg-url"
          hint="双向即时转换，编码结果可直接用于 CSS background-image。"
          left={{ label: "SVG 源码", placeholder: "粘贴 SVG 源码" }}
          right={{ label: "URL 编码（data URI）", placeholder: "粘贴 data URI 或编码后的文本" }}
          toRight={svgToDataUri}
          toLeft={dataUriToSvg}
          extra={(svg) => <SvgIconPanel svg={svg} />}
        />
      )}
      {active === "base64" && (
        <InstantConverter
          stateKey="encoding.base64"
          hint="双向即时转换，按 UTF-8 编解码，兼容 base64url 与换行分隔的输入。"
          left={{ label: "文本", placeholder: "输入或粘贴文本" }}
          right={{ label: "Base64", placeholder: "粘贴 Base64 字符串" }}
          toRight={textToBase64}
          toLeft={base64ToText}
        />
      )}
      {active === "query" && <QueryTableTool />}
    </div>
  );
};

/** 转换器一侧的标签与占位提示。 */
type ConverterSide = { label: string; placeholder: string };

/**
 * 双栏即时转换器：编辑任意一侧，另一侧立即更新。
 * 左 → 右恒成功；右 → 左可能失败，失败时保留左栏旧值并显示错误条。
 * 左右两侧与错误态经 localStorage 持久化（子 tab 切换卸载/重挂不丢现场）。
 */
const InstantConverter = ({
  stateKey,
  hint,
  left,
  right,
  toRight,
  toLeft,
  extra,
}: {
  stateKey: string;
  hint: string;
  left: ConverterSide;
  right: ConverterSide;
  toRight: (value: string) => string;
  toLeft: (value: string) => ConvertResult<string>;
  /** 挂在双栏下方的附加面板，入参为当前 SVG 源码一侧的内容。 */
  extra?: (leftText: string) => ReactNode;
}) => {
  const [leftText, setLeftText] = usePersistedToolState(`${stateKey}.left`, "", restoreString);
  const [rightText, setRightText] = usePersistedToolState(`${stateKey}.right`, "", restoreString);
  const [error, setError] = usePersistedToolState<string | null>(
    `${stateKey}.error`,
    null,
    restoreNullableString,
  );

  const handleLeftChange = (value: string) => {
    setLeftText(value);
    setRightText(toRight(value));
    setError(null);
  };

  const handleRightChange = (value: string) => {
    setRightText(value);
    const result = toLeft(value);
    if (result.ok) {
      setLeftText(result.value);
      setError(null);
    } else {
      setError(result.message);
    }
  };

  // 清空任一侧即整体归零：双向即时转换下旧内容不再有意义，错误态一并清除
  const clearBoth = () => {
    setLeftText("");
    setRightText("");
    setError(null);
  };

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-4">
      <p className="text-sm text-muted-foreground">{hint}</p>
      {error && <FeedbackStrip kind="error">{error}</FeedbackStrip>}
      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <LabeledTextarea
          label={left.label}
          value={leftText}
          onChange={(event) => handleLeftChange(event.target.value)}
          placeholder={left.placeholder}
          action={
            <>
              <CopyButton text={leftText} variant="ghost" />
              <ClearButton
                label={`清空${left.label}`}
                onClick={clearBoth}
                disabled={leftText === ""}
              />
            </>
          }
        />
        <LabeledTextarea
          label={right.label}
          value={rightText}
          onChange={(event) => handleRightChange(event.target.value)}
          placeholder={right.placeholder}
          action={
            <>
              <CopyButton text={rightText} variant="ghost" />
              <ClearButton
                label={`清空${right.label}`}
                onClick={clearBoth}
                disabled={rightText === ""}
              />
            </>
          }
        />
      </div>
      {extra?.(leftText)}
    </section>
  );
};

/**
 * SVG ↔ URL 子工具的预览与 iconify/scss 模板。预览经 <img> 渲染 data URI——
 * 与最终 CSS background-image 同为图像语境，所见即所得；非法 SVG 由 img 的
 * error 事件呈错误态。模板 data URI 复用现有编码输出。
 */
const SvgIconPanel = ({ svg }: { svg: string }) => {
  const [iconName, setIconName] = usePersistedToolState(
    "encoding.svg-icon-name",
    "",
    restoreString,
  );
  const [broken, setBroken] = useState(false);
  const dataUri = svgToDataUri(svg);

  // src 变化后重置错误态：img 的 error 只在加载时报告一次
  useEffect(() => {
    setBroken(false);
  }, [dataUri]);

  const template = svgIconScss(dataUri, iconName);

  return (
    <section className="grid shrink-0 grid-cols-[auto_minmax(0,1fr)] items-start gap-4 rounded-md border p-3">
      <figure className="flex h-24 w-40 items-center justify-center overflow-hidden rounded border bg-card p-2">
        {svg === "" ? (
          <figcaption className="text-xs text-muted-foreground">粘贴 SVG 源码后预览</figcaption>
        ) : broken ? (
          <figcaption className="text-xs text-destructive">无法渲染预览：SVG 源码不合法</figcaption>
        ) : (
          <img
            src={dataUri}
            alt="SVG 预览"
            onError={() => setBroken(true)}
            onLoad={() => setBroken(false)}
            className="max-h-full max-w-full object-contain"
          />
        )}
      </figure>
      <div className="flex min-w-0 flex-col gap-2">
        <div className="flex items-center gap-2">
          <label htmlFor="svg-icon-name" className="text-sm font-medium text-muted-foreground">
            iconName
          </label>
          <Input
            id="svg-icon-name"
            value={iconName}
            onChange={(event) => setIconName(event.target.value)}
            placeholder="icon"
            className="h-8 w-40 font-mono text-sm"
            spellCheck={false}
          />
          <CopyButton text={svg === "" ? "" : template} variant="ghost" />
        </div>
        <pre className="max-h-24 min-w-0 overflow-y-auto rounded border bg-card p-2 font-mono text-xs break-all whitespace-pre-wrap text-muted-foreground">
          {template}
        </pre>
      </div>
    </section>
  );
};

type QueryRow = QueryParam & { id: string };

const QueryTableTool = () => {
  const [query, setQuery] = usePersistedToolState("encoding.query", "", restoreString);
  const [error, setError] = usePersistedToolState<string | null>(
    "encoding.query.error",
    null,
    restoreNullableString,
  );
  // 键值表是 query 的派生视图：恢复时重新解析生成，行 id 每次会话重新分配
  const [rows, setRows] = useState<QueryRow[]>(() => {
    if (query === "") return [];
    const result = parseQuery(query);
    return result.ok ? result.value.map((param) => ({ ...param, id: crypto.randomUUID() })) : [];
  });

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
      {error && <FeedbackStrip kind="error">{error}</FeedbackStrip>}
      <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
        <LabeledTextarea
          label="Query 字符串"
          value={query}
          onChange={(event) => handleQueryChange(event.target.value)}
          placeholder="粘贴 query 字符串或完整 URL"
          action={
            <>
              <CopyButton text={query} variant="ghost" />
              <ClearButton
                label="清空 Query 字符串"
                onClick={() => handleQueryChange("")}
                disabled={query === ""}
              />
            </>
          }
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
};

export const Route = createFileRoute("/_main/encoding")({
  component: EncodingToolPage,
});
