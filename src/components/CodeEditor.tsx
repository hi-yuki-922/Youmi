import * as monaco from "monaco-api";
import "monaco-json-contribution";
import * as React from "react";
import { useEffect, useRef, useState } from "react";

// monaco workers 本地打包（桌面离线场景禁用 CDN loader）：经本地 wrapper 文件
// 作 new URL 锚点打包成 worker chunk。JSON 语法校验走 json worker，其余编辑
// 服务走 editor worker
(self as unknown as { MonacoEnvironment: monaco.Environment }).MonacoEnvironment = {
  getWorker: (_workerId: string, label: string) =>
    label === "json"
      ? new Worker(new URL("../monaco/json.worker.ts", import.meta.url), { type: "module" })
      : new Worker(new URL("../monaco/editor.worker.ts", import.meta.url), { type: "module" }),
};

/** 跟随系统明暗切换主题（应用无应用内主题开关，统一随 prefers-color-scheme）。 */
const useFollowSystemTheme = () => {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => monaco.editor.setTheme(media.matches ? "vs-dark" : "vs");
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, []);
};

interface CodeEditorProps {
  value: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  /** 值为空时的占位文案（monaco 无原生 placeholder，空态叠加渲染）。 */
  placeholder?: string;
  ariaLabel: string;
}

/** 受控 monaco 编辑器：编辑器实例只建一次，外部值变化同步进去（内容一致不动，
 * 避免清空/回填时光标跳动），内部输入经 onChange 回流。 */
const CodeEditor = ({
  value,
  onChange,
  readOnly = false,
  placeholder,
  ariaLabel,
}: CodeEditorProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  // onChange 经 ref 转发：内容订阅只建一次，回调换新不重建编辑器
  const changeRef = useRef(onChange);
  changeRef.current = onChange;
  const [focused, setFocused] = useState(false);

  useFollowSystemTheme();

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const editor = monaco.editor.create(container, {
      value,
      language: "json",
      readOnly,
      automaticLayout: true, // 容器随窗口伸缩
      minimap: { enabled: false },
      wordWrap: "on",
      scrollBeyondLastLine: false,
      fontSize: 13,
      fixedOverflowWidgets: true,
      scrollbar: { verticalScrollbarSize: 10, horizontalScrollbarSize: 10 },
    });
    editorRef.current = editor;
    editor.onDidFocusEditorText(() => setFocused(true));
    editor.onDidBlurEditorText(() => setFocused(false));
    const subscription = editor.onDidChangeModelContent(() => {
      changeRef.current?.(editor.getValue());
    });
    return () => {
      subscription.dispose();
      editor.getModel()?.dispose();
      editor.dispose();
      editorRef.current = null;
    };
    // 编辑器生命周期只跟挂载绑定，值/只读经下方 effect 同步
  }, []);

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && editor.getValue() !== value) editor.setValue(value);
  }, [value]);

  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly });
  }, [readOnly]);

  return (
    <div
      className="relative min-h-0 min-w-0 flex-1 overflow-hidden rounded-md border bg-background"
      role="textbox"
      aria-multiline="true"
      aria-label={ariaLabel}
    >
      {!value && !focused && placeholder && (
        <p className="pointer-events-none absolute left-3 top-2 z-10 select-none text-sm text-muted-foreground">
          {placeholder}
        </p>
      )}
      <div ref={containerRef} className="absolute inset-0" />
    </div>
  );
};

/** 带标签的代码编辑面板：工作台工具双栏布局的输入/结果区，标签行右侧可挂动作。 */
const LabeledCodeEditor = ({
  label,
  action,
  ...props
}: CodeEditorProps & { label: string; action?: React.ReactNode }) => (
  <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
    {/* min-h-8：无 action 的一侧与按钮同高，保证双栏顶边对齐 */}
    <div className="flex min-h-8 items-center justify-between gap-2">
      <span className="text-sm font-medium text-muted-foreground">{label}</span>
      {action && <div className="flex items-center gap-1">{action}</div>}
    </div>
    <CodeEditor {...props} />
  </div>
);

export { LabeledCodeEditor };
