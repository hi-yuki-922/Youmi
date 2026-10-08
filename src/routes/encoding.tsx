import { createFileRoute } from "@tanstack/react-router";

import { ToolPlaceholder } from "@/components/layout/placeholder";

export const Route = createFileRoute("/encoding")({
  component: () => (
    <ToolPlaceholder
      title="编码转换"
      description="SVG 与 URL 编码、Base64、URL query 互转——即将上线"
    />
  ),
});
