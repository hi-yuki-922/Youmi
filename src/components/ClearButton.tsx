import { BrushCleaning } from "lucide-react";

import { Button } from "@/components/ui/Button";

/** 一键清空输入的图标按钮（BrushCleaning）：置于输入框 label 行动作位，直接清空不做确认。 */
export const ClearButton = ({
  label,
  onClick,
  disabled,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
}) => (
  <Button
    variant="ghost"
    size="icon"
    aria-label={label}
    title={label}
    onClick={onClick}
    disabled={disabled}
    className="size-8 shrink-0 text-muted-foreground"
  >
    <BrushCleaning />
  </Button>
);
