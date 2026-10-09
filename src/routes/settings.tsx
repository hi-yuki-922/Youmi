import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState, type ReactNode } from "react";
import * as autostart from "@tauri-apps/plugin-autostart";

import { Switch } from "@/components/ui/switch";
import { getAutostartPreference, setAutostartPreference } from "@/lib/settings";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-medium text-muted-foreground">{title}</h2>
      <div className="divide-y rounded-lg border bg-card">{children}</div>
    </section>
  );
}

function SettingRow({
  title,
  description,
  control,
}: {
  title: string;
  description?: string;
  control: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm">{title}</span>
        {description && <span className="text-xs text-muted-foreground">{description}</span>}
      </div>
      {control}
    </div>
  );
}

function AutostartRow() {
  const [enabled, setEnabled] = useState(false);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let active = true;
    void (async () => {
      // 实际状态以插件（OS 注册表）为准；若 store 记录与之不符，一并校正
      const actual = await autostart.isEnabled();
      const recorded = await getAutostartPreference();
      if (!active) return;
      setEnabled(actual);
      setReady(true);
      if (recorded !== actual) await setAutostartPreference(actual);
    })().catch(() => {
      if (active) setReady(true);
    });
    return () => {
      active = false;
    };
  }, []);

  const handleToggle = async (next: boolean) => {
    try {
      if (next) {
        await autostart.enable();
      } else {
        await autostart.disable();
      }
      await setAutostartPreference(next);
      setEnabled(next);
    } catch (error) {
      console.error("切换开机自启动失败", error);
      autostart
        .isEnabled()
        .then(setEnabled)
        .catch(() => {});
    }
  };

  return (
    <SettingRow
      title="开机自启动"
      description="登录 Windows 后自动启动 Youmi"
      control={
        <Switch
          checked={enabled}
          disabled={!ready}
          onCheckedChange={(next) => void handleToggle(next)}
        />
      }
    />
  );
}

function SettingsPage() {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <h1 className="text-xl font-semibold">设置</h1>
      <SettingsSection title="通用">
        <AutostartRow />
      </SettingsSection>
    </div>
  );
}
