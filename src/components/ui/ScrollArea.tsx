import * as ScrollAreaPrimitive from "@radix-ui/react-scroll-area";
import * as React from "react";

import { cn } from "@/lib/utils";

/**
 * radix ScrollArea：原生滚动条被隐藏，滚动条样式统一由 Thumb 呈现
 * （默认 hover 时浮现）。工具页根元素都是 `h-full` 布局，Content 以
 * display:block 覆盖 radix 的 display:table 并 grow 撑满——内容不足时
 * 填满视口（h-full 页面可解析），超高时随内容增长、滚动归 viewport。
 */
const ScrollArea = ({
  className,
  children,
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Root>) => (
  <ScrollAreaPrimitive.Root
    data-slot="scroll-area"
    className={cn("relative overflow-hidden", className)}
    {...props}
  >
    <ScrollAreaPrimitive.Viewport
      data-slot="scroll-area-viewport"
      // disableImplicitContentElement：Viewport 不再包一层隐式 content div
      // （display:table、auto 高度），下面的 Content 才是唯一内容层与测量目标
      disableImplicitContentElement
      className="flex size-full flex-col rounded-[inherit] outline-none"
    >
      <ScrollAreaPrimitive.Content
        data-slot="scroll-area-content"
        style={{ display: "block" }}
        className="grow"
      >
        {children}
      </ScrollAreaPrimitive.Content>
    </ScrollAreaPrimitive.Viewport>
    <ScrollBar />
    <ScrollAreaPrimitive.Corner />
  </ScrollAreaPrimitive.Root>
);

const ScrollBar = ({
  className,
  orientation = "vertical",
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Scrollbar>) => (
  <ScrollAreaPrimitive.Scrollbar
    data-slot="scroll-area-scrollbar"
    orientation={orientation}
    className={cn(
      "flex touch-none select-none p-px transition-colors",
      orientation === "vertical" && "h-full w-2.5 border-l border-l-transparent",
      orientation === "horizontal" && "h-2.5 flex-col border-t border-t-transparent",
      className,
    )}
    {...props}
  >
    <ScrollAreaPrimitive.Thumb
      data-slot="scroll-area-thumb"
      className="relative flex-1 rounded-full bg-border"
    />
  </ScrollAreaPrimitive.Scrollbar>
);

export { ScrollArea, ScrollBar };
