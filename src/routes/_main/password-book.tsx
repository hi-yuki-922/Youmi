import { createFileRoute } from "@tanstack/react-router";

import { ToolPlaceholder } from "@/components/layout/Placeholder";

export const Route = createFileRoute("/_main/password-book")({
  component: () => (
    <ToolPlaceholder title="密码本" description="以主密码解锁的凭据条目集合——即将上线" />
  ),
});
