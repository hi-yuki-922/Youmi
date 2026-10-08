import { createFileRoute } from "@tanstack/react-router";

import { ToolPlaceholder } from "@/components/layout/placeholder";

export const Route = createFileRoute("/diff")({
  component: () => (
    <ToolPlaceholder
      title="文本 diff"
      description="两段文本并排与统一对比，行级/行内高亮——即将上线"
    />
  ),
});
