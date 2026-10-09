import { cn } from "@/lib/utils";

/** 工作台页共用的分段式 tab 切换（role=tablist）。 */
export function SegmentedTabs<T extends string>({
  "aria-label": ariaLabel,
  options,
  value,
  onChange,
  className,
}: {
  "aria-label": string;
  options: ReadonlyArray<{ id: T; label: string }>;
  value: T;
  onChange: (value: T) => void;
  className?: string;
}) {
  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={cn("flex w-fit gap-1 rounded-lg bg-muted p-1", className)}
    >
      {options.map((option) => (
        <button
          key={option.id}
          type="button"
          role="tab"
          aria-selected={value === option.id}
          onClick={() => onChange(option.id)}
          className={cn(
            "rounded-md px-3 py-1.5 text-sm transition-colors",
            value === option.id
              ? "bg-background font-medium text-foreground shadow-xs"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
