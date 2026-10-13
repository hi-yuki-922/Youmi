import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState, type ReactNode } from "react";
import * as autostart from "@tauri-apps/plugin-autostart";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Switch } from "@/components/ui/Switch";
import { getClipboardCapacity, setClipboardCapacity } from "@/lib/clipboard";
import { getAutostartPreference, setAutostartPreference } from "@/lib/settings";
import {
  DEEPSEEK_DEFAULT_MODEL,
  getTranslateApiKey,
  getTranslateModel,
  setTranslateApiKey,
  setTranslateModel,
} from "@/lib/translate";

const SettingsSection = ({ title, children }: { title: string; children: ReactNode }) => (
  <section className="flex flex-col gap-2">
    <h2 className="text-sm font-medium text-muted-foreground">{title}</h2>
    <div className="divide-y rounded-lg border bg-card">{children}</div>
  </section>
);

const SettingRow = ({
  title,
  description,
  control,
}: {
  title: string;
  description?: string;
  control?: ReactNode;
}) => {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm">{title}</span>
        {description && <span className="text-xs text-muted-foreground">{description}</span>}
      </div>
      {control}
    </div>
  );
};

const AutostartRow = () => {
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
};

/** 容量输入失焦/回车才提交；以引擎 clamp 后的值回显，非法输入恢复原值。 */
const ClipboardCapacityRow = () => {
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
};

const SettingsPage = () => {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <h1 className="text-xl font-semibold">设置</h1>
      <SettingsSection title="通用">
        <AutostartRow />
      </SettingsSection>
      <SettingsSection title="剪贴板">
        <ClipboardCapacityRow />
        <SettingRow
          title="呼不出剪贴板浮层？"
          description="按 Alt+V 呼出/收起剪贴板浮层；无效时通常是 Alt+V 被其他程序占用"
        />
      </SettingsSection>
      <SettingsSection title="划词翻译">
        <DeepSeekApiKeyRow />
        <DeepSeekModelRow />
        <SettingRow
          title="呼不出翻译浮层？"
          description="在任意应用选中文字后按 Alt+T 翻译；无效时通常是 Alt+T 被其他程序占用"
        />
      </SettingsSection>
    </div>
  );
};

/** API key 存系统凭据库（Rust command），页面不回显明文，仅显示已配置状态。 */
const DeepSeekApiKeyRow = () => {
  const [value, setValue] = useState("");
  const [configured, setConfigured] = useState(false);
  const [ready, setReady] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void getTranslateApiKey()
      .then((key) => {
        if (!active) return;
        setConfigured(Boolean(key));
        setReady(true);
      })
      .catch((error) => {
        console.error("读取 DeepSeek API key 失败", error);
        if (active) setReady(true);
      });
    return () => {
      active = false;
    };
  }, []);

  const save = async () => {
    if (!ready || saving) return;
    setSaving(true);
    try {
      await setTranslateApiKey(value);
      setConfigured(Boolean(value.trim()));
      setValue("");
    } catch (error) {
      console.error("保存 DeepSeek API key 失败", error);
    } finally {
      setSaving(false);
    }
  };

  return (
    <SettingRow
      title="DeepSeek API Key"
      description={
        configured
          ? "已保存到系统凭据库；输入新值替换，留空保存即清除"
          : "存储在系统凭据库，不落明文文件"
      }
      control={
        <div className="flex items-center gap-2">
          <Input
            type="password"
            value={value}
            placeholder={configured ? "••••••••" : "sk-…"}
            disabled={!ready || saving}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void save();
            }}
            className="w-56"
          />
          <Button
            variant="outline"
            size="sm"
            disabled={!ready || saving}
            onClick={() => void save()}
          >
            保存
          </Button>
        </div>
      }
    />
  );
};

/** 模型名失焦/回车才提交；非秘密，存设置文件。 */
const DeepSeekModelRow = () => {
  const [value, setValue] = useState("");
  const [committed, setCommitted] = useState("");
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let active = true;
    void getTranslateModel().then((model) => {
      if (!active) return;
      setValue(model);
      setCommitted(model);
      setReady(true);
    });
    return () => {
      active = false;
    };
  }, []);

  const commit = async () => {
    if (!ready) return;
    const trimmed = value.trim();
    if (!trimmed) {
      setValue(committed);
      return;
    }
    if (trimmed === committed) return;
    try {
      await setTranslateModel(trimmed);
      setCommitted(trimmed);
      setValue(trimmed);
    } catch (error) {
      console.error("保存模型名失败", error);
      setValue(committed);
    }
  };

  return (
    <SettingRow
      title="模型名"
      description={`DeepSeek 开放平台的模型 ID，默认 ${DEEPSEEK_DEFAULT_MODEL}`}
      control={
        <Input
          value={value}
          placeholder={DEEPSEEK_DEFAULT_MODEL}
          disabled={!ready}
          onChange={(e) => setValue(e.target.value)}
          onBlur={() => void commit()}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
          className="w-56 text-right"
        />
      }
    />
  );
};

export const Route = createFileRoute("/_main/settings")({
  component: SettingsPage,
});
