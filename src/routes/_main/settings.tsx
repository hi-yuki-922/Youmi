import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState, type ReactNode } from "react";
import * as autostart from "@tauri-apps/plugin-autostart";

import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { getClipboardCapacity, setClipboardCapacity } from "@/lib/clipboard";
import { getAutostartPreference, setAutostartPreference } from "@/lib/settings";

export const Route = createFileRoute("/_main/settings")({
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
  control?: ReactNode;
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

/** 容量输入失焦/回车才提交；以引擎 clamp 后的值回显，非法输入恢复原值。 */
function ClipboardCapacityRow() {
  const [value, setValue] = useState("");
  const [committed, setCommitted] = useState<number | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let active = true;
    void getClipboardCapacity().then((capacity) => {
      if (!active) return;
      setValue(String(capacity));
      setCommitted(capacity);
      setReady(true);
    });
    return () => {
      active = false;
    };
  }, []);

  const commit = async () => {
    if (!ready) return;
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed < 1) {
      setValue(String(committed));
      return;
    }
    if (parsed === committed) return;
    try {
      await setClipboardCapacity(parsed);
      const applied = await getClipboardCapacity();
      setCommitted(applied);
      setValue(String(applied));
    } catch (error) {
      console.error("修改剪贴板历史容量失败", error);
      setValue(String(committed));
    }
  };

  return (
    <SettingRow
      title="历史容量"
      description="保留的最多条数，超出后自动淘汰最旧条目"
      control={
        <div className="flex items-center gap-2">
          <Input
            type="number"
            min={1}
            value={value}
            disabled={!ready}
            onChange={(e) => setValue(e.target.value)}
            onBlur={() => void commit()}
            onKeyDown={(e) => {
              if (e.key === "Enter") e.currentTarget.blur();
            }}
            className="w-24 text-right"
          />
          <span className="text-sm text-muted-foreground">条</span>
        </div>
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
      <SettingsSection title="剪贴板">
        <ClipboardCapacityRow />
        <SettingRow
          title="Win+V 呼不出浮层？"
          description="系统剪贴板历史占用了 Win+V：请在 Windows 设置 → 系统 → 剪贴板 中关闭「剪贴板历史」后重启 Youmi"
        />
      </SettingsSection>
    </div>
  );
}
