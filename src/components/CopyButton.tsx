import { type VariantProps } from "class-variance-authority";
import { Check, Copy } from "lucide-react";
import { useState } from "react";

import { Button, buttonVariants } from "@/components/ui/Button";

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
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <Button variant={variant} size={size} onClick={handleCopy} disabled={disabled}>
      {copied ? <Check data-slot="icon" /> : <Copy data-slot="icon" />}
      {copied ? "已复制" : "复制结果"}
    </Button>
  );
};
