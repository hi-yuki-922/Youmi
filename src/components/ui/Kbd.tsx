import * as React from "react";

import { comboSymbols, formatCombo } from "@/lib/kbd";
import { cn } from "@/lib/utils";

const Kbd = ({ className, ...props }: React.ComponentProps<"kbd">) => (
  <kbd
    data-slot="kbd"
    className={cn(
      "inline-flex h-5 min-w-5 select-none items-center justify-center rounded-sm border bg-muted px-1 font-sans text-xs font-medium text-muted-foreground",
      className,
    )}
    {...props}
  />
);

const KbdGroup = ({ className, ...props }: React.ComponentProps<"div">) => (
  <div
    data-slot="kbd-group"
    className={cn("inline-flex items-center gap-1", className)}
    {...props}
  />
);

/** 组合键规范串 → 一组 Kbd（"alt+KeyV" → ⌘ V）；文本形式作 aria-label 供读屏。 */
const ComboKbd = ({ combo, className }: { combo: string; className?: string }) => (
  <KbdGroup aria-label={formatCombo(combo)} className={className}>
    {comboSymbols(combo).map((symbol) => (
      <Kbd key={symbol}>{symbol}</Kbd>
    ))}
  </KbdGroup>
);

export { ComboKbd, Kbd, KbdGroup };
