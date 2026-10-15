import { type VariantProps } from "class-variance-authority";
import { Check, Copy } from "lucide-react";
import { useState } from "react";

import { Button, buttonVariants } from "@/components/ui/Button";
import { writeClipboardText } from "@/lib/clipboard";

type CopyButtonProps = {
  text: string;
  disabled?: boolean;
} & Pick<VariantProps<typeof buttonVariants>, "variant" | "size">;

export const CopyButton = ({
  text,
  disabled = text === "",
  variant,
  size = "sm",
}: CopyButtonProps) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    if (disabled) return;
    try {
      await writeClipboardText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (cause) {
      console.error("复制失败", cause);
    }
  };

  return (
    <Button variant={variant} size={size} onClick={handleCopy} disabled={disabled}>
      {copied ? <Check data-slot="icon" /> : <Copy data-slot="icon" />}
      {copied ? "已复制" : "复制结果"}
    </Button>
  );
};
