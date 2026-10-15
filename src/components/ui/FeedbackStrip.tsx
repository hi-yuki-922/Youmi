import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** 结果反馈条：success / error 两态的行内提示，工具页共用。 */
export const FeedbackStrip = ({
  kind,
  children,
}: {
  kind: "success" | "error";
  children: ReactNode;
}) => (
  <div
    className={cn(
      "rounded-md px-3 py-2 text-sm",
      kind === "success" ? "bg-success/10 text-success" : "bg-destructive/10 text-destructive",
    )}
  >
    {children}
  </div>
);
