import * as React from "react";
import { useId } from "react";

import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

/**
 * 带标签的等宽多行文本面板：工作台工具双栏布局的输入/结果区。
 * `action` 渲染在标签行右侧（如复制按钮）。
 */
function LabeledTextarea({
  label,
  action,
  className,
  id,
  ...props
}: React.ComponentProps<"textarea"> & { label: string; action?: React.ReactNode }) {
  const autoId = useId();
  const textareaId = id ?? autoId;

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <label htmlFor={textareaId} className="text-sm font-medium text-muted-foreground">
          {label}
        </label>
        {action}
      </div>
      <Textarea
        id={textareaId}
        spellCheck={false}
        className={cn("min-h-0 flex-1 resize-none font-mono text-sm", className)}
        {...props}
      />
    </div>
  );
}

export { LabeledTextarea };
