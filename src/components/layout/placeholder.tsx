import { Construction } from "lucide-react";

export function ToolPlaceholder({ title, description }: { title: string; description: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-muted-foreground">
      <Construction className="size-10" />
      <h1 className="text-xl font-semibold text-foreground">{title}</h1>
      <p className="text-sm">{description}</p>
    </div>
  );
}
