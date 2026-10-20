import { ChevronDown } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { Input } from "@/components/ui/Input";
import { currencyName, type CurrencyInfo } from "@/tools/exchange/core";
import { cn } from "@/lib/utils";

/**
 * 可搜索币种下拉：按钮触发浮层（搜索框 + 代码/中文名过滤列表），键盘可导航。
 * `value` 为空时按钮呈"添加"形态（`addLabel`），用于目标币种行的追加入口。
 */
export const CurrencyPicker = ({
  value,
  options,
  onChange,
  addLabel,
  ariaLabel,
}: {
  value: string;
  options: CurrencyInfo[];
  onChange: (code: string) => void;
  /** value 为空时按钮文案（添加形态）；未提供则空值显示占位符。 */
  addLabel?: string;
  ariaLabel: string;
}) => {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const boxRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  // 打开即聚焦搜索框；点击浮层之外关闭
  useEffect(() => {
    if (!open) return;
    searchRef.current?.focus();
    const onPointerDown = (event: PointerEvent) => {
      if (!boxRef.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    return () => window.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q === "") return options;
    return options.filter(
      (option) => option.code.toLowerCase().includes(q) || option.name.includes(query.trim()),
    );
  }, [options, query]);

  const openPanel = () => {
    setQuery("");
    setActive(0);
    setOpen(true);
  };

  const pick = (code: string) => {
    onChange(code);
    setOpen(false);
  };

  const activeIndex = Math.min(active, filtered.length - 1);

  return (
    <div ref={boxRef} className="relative">
      <button
        type="button"
        aria-label={ariaLabel}
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : openPanel())}
        className={cn(
          "flex h-8 items-center gap-1.5 rounded-md border px-2.5 text-sm",
          "hover:bg-accent/50",
          value === "" && "text-muted-foreground",
        )}
      >
        {value === "" ? (
          <span>{addLabel ?? "选择币种"}</span>
        ) : (
          <>
            <span className="font-mono">{value}</span>
            <span className="text-muted-foreground">{currencyName(value)}</span>
          </>
        )}
        <ChevronDown className="size-3.5 text-muted-foreground" />
      </button>

      {open && (
        <div className="absolute left-0 top-9 z-20 w-64 rounded-md border bg-popover shadow-md">
          <div className="border-b p-2">
            <Input
              ref={searchRef}
              role="combobox"
              aria-controls="currency-options"
              aria-activedescendant={
                filtered[activeIndex] ? `currency-option-${activeIndex}` : undefined
              }
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setActive(0);
              }}
              onKeyDown={(event) => {
                if (event.key === "ArrowDown") {
                  event.preventDefault();
                  setActive(Math.min(activeIndex + 1, filtered.length - 1));
                } else if (event.key === "ArrowUp") {
                  event.preventDefault();
                  setActive(Math.max(activeIndex - 1, 0));
                } else if (event.key === "Enter") {
                  event.preventDefault();
                  const option = filtered[activeIndex];
                  if (option) pick(option.code);
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  setOpen(false);
                }
              }}
              placeholder="搜索代码或中文名"
              className="h-7 text-sm"
            />
          </div>
          <ul
            id="currency-options"
            role="listbox"
            aria-label={ariaLabel}
            className="max-h-64 overflow-y-auto py-1"
          >
            {filtered.length === 0 ? (
              <li className="px-3 py-2 text-sm text-muted-foreground">没有匹配的币种</li>
            ) : (
              filtered.map((option, index) => (
                <li
                  key={option.code}
                  id={`currency-option-${index}`}
                  role="option"
                  aria-selected={index === activeIndex}
                  onMouseEnter={() => setActive(index)}
                  onClick={() => pick(option.code)}
                  className={cn(
                    "flex cursor-default items-center gap-2 px-3 py-1.5 text-sm",
                    index === activeIndex && "bg-accent/70",
                  )}
                >
                  <span className="w-12 shrink-0 font-mono">{option.code}</span>
                  <span className="min-w-0 flex-1 truncate">{option.name}</span>
                </li>
              ))
            )}
          </ul>
        </div>
      )}
    </div>
  );
};
