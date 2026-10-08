import { createFileRoute } from "@tanstack/react-router";

import { ToolPlaceholder } from "@/components/layout/placeholder";

export const Route = createFileRoute("/settings")({
  component: () => (
    <ToolPlaceholder title="设置" description="自启动、快捷键、翻译模型、剪贴板容量——即将上线" />
  ),
});
