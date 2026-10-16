import { createFileRoute } from "@tanstack/react-router";
import { SquarePen } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import * as autostart from "@tauri-apps/plugin-autostart";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { ComboKbd } from "@/components/ui/Kbd";
import { Switch } from "@/components/ui/Switch";
import {
  getClipboardCapacity,
  getClipboardEngineAvailable,
  setClipboardCapacity,
} from "@/lib/clipboard";
import {
  beginShortcutRecord,
  cancelShortcutRecord,
  getShortcuts,
  hasRequiredModifier,
  onShortcutRecord,
  setShortcut,
  type ShortcutAction,
  type ShortcutBinding,
} from "@/lib/shortcuts";
import { getAutostartPreference, setAutostartPreference } from "@/lib/settings";
import {
  DEEPSEEK_DEFAULT_MODEL,
  getTranslateModel,
  hasTranslateApiKey,
  setTranslateApiKey,
  setTranslateModel,
} from "@/lib/translate";
import { getVaultLockTimeoutMinutes, setVaultLockTimeoutMinutes } from "@/lib/vault";
import { QUICK_PASTEBACK_SLOTS } from "@/tools/clipboard/core";

const SettingsSection = ({ title, children }: { title: string; children: ReactNode }) => (
  <section className="flex flex-col gap-2">
    <h2 className="text-sm font-medium text-muted-foreground">{title}</h2>
    <div className="divide-y rounded-lg border bg-card">{children}</div>
  </section>
);

const SettingRow = ({
  title,
  tag,
  description,
  error,
  control,
}: {
  title: string;
  tag?: string;
  description?: string;
  error?: string;
  control?: ReactNode;
}) => {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <span className="flex items-center gap-1.5 text-sm">
          {title}
          {tag && (
            <span className="rounded-sm bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
              {tag}
            </span>
          )}
        </span>
        {description && <span className="text-xs text-muted-foreground">{description}</span>}
        {error && <span className="text-xs text-destructive">{error}</span>}
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

const SHORTCUT_ACTION_LABELS: Record<ShortcutAction, string> = {
  clipboard: "剪贴板浮层",
  palette: "命令面板",
  translate: "划词翻译",
};

const SHORTCUT_DESCRIPTIONS: Record<ShortcutAction, string> = {
  clipboard: "呼出/收起剪贴板浮层；呼不出时通常是组合键被其他程序占用",
  palette: "呼出/收起命令面板；呼不出时通常是组合键被其他程序占用",
  translate: "选中文字后呼出划词翻译；呼不出时通常是组合键被其他程序占用",
};

/** 剪贴板浮层内生效的只读快捷键：Alt+1~N 直达回帖，不经系统注册、不可改键。 */
const CLIPBOARD_SCOPE_SHORTCUTS: ReadonlyArray<{
  combo: string;
  title: string;
  description: string;
}> = Array.from({ length: QUICK_PASTEBACK_SLOTS }, (_, i) => i + 1).map((n) => ({
  combo: `alt+Digit${n}`,
  title: `回帖第 ${n} 条`,
  description: `快速回帖浮层内当前可见（含搜索过滤后）列表的第 ${n} 条`,
}));

/** 快捷键生效作用域：全局 = 系统级注册、任意应用生效；剪贴板 = 浮层窗口内生效。 */
type ShortcutScope = "global" | "clipboard";

const SCOPE_META: Record<ShortcutScope, { label: string; description: string }> = {
  global: { label: "全局", description: "系统级注册，任意应用内生效" },
  clipboard: { label: "剪贴板", description: "剪贴板浮层窗口内生效，不经系统注册" },
};

/** 作用域分组卡片：组头标注作用域与生效范围，条目行以分隔线排列。 */
const ScopeGroupCard = ({ scope, children }: { scope: ShortcutScope; children: ReactNode }) => {
  const meta = SCOPE_META[scope];
  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      <div className="border-b bg-muted/40 px-4 py-2">
        <span className="text-xs font-medium">{meta.label}</span>
        <span className="ml-2 text-xs text-muted-foreground">{meta.description}</span>
      </div>
      <div className="divide-y">{children}</div>
    </div>
  );
};

/** 快捷键行：Kbd 呈现组合键；图标按钮进入录制，录制中按钮隐藏（一次只有一个录制会话）。 */
const ShortcutRow = ({
  scope,
  binding,
  recording,
  error,
  onRecordStart,
}: {
  scope: ShortcutScope;
  binding: ShortcutBinding;
  recording: boolean;
  error?: string;
  onRecordStart: () => void;
}) => (
  <SettingRow
    title={SHORTCUT_ACTION_LABELS[binding.action]}
    tag={SCOPE_META[scope].label}
    description={
      recording ? "点击任意位置或按 Esc 取消录制" : SHORTCUT_DESCRIPTIONS[binding.action]
    }
    error={
      error ??
      (binding.registered
        ? undefined
        : "该组合键当前注册失败（可能被系统或其他程序占用），重新修改可重试")
    }
    control={
      recording ? (
        <span className="text-sm text-muted-foreground">按下新组合键…</span>
      ) : (
        <div className="flex items-center gap-2">
          <ComboKbd combo={binding.combo} />
          <Button
            variant="ghost"
            size="icon"
            aria-label="修改快捷键"
            title="修改快捷键"
            onClick={onRecordStart}
            className="size-8 shrink-0 text-muted-foreground"
          >
            <SquarePen />
          </Button>
        </div>
      )
    }
  />
);

const ShortcutsSection = () => {
  const [bindings, setBindings] = useState<ShortcutBinding[] | null>(null);
  const [recording, setRecording] = useState<ShortcutAction | null>(null);
  const [error, setError] = useState<{ action: ShortcutAction; message: string } | null>(null);

  const load = useCallback(() => {
    void getShortcuts()
      .then(setBindings)
      .catch((cause: unknown) => console.error("读取快捷键设置失败", cause));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // 录制会话结果经系统级事件回来：钩子在 Rust 侧截获按键（Alt+Space 等
  // OS 保留组合的 keydown 根本到不了页面），Esc/超时/取消以 cancelled 结束
  const recordingRef = useRef<ShortcutAction | null>(null);
  useEffect(() => {
    recordingRef.current = recording;
  }, [recording]);

  useEffect(() => {
    const unlisten = onShortcutRecord((event) => {
      const action = recordingRef.current;
      setRecording(null);
      if (!action || event.kind === "cancelled") return;
      // 超时未截获到按键：多半是按键被其他软件的键盘钩子拦走，如实提示
      if (event.kind === "timeout") {
        setError({
          action,
          message: "10 秒内未截获到按键（可能被其他软件拦截），已结束录制，请重试",
        });
        return;
      }
      if (!hasRequiredModifier(event.combo)) {
        setError({ action, message: "组合键需包含 Ctrl、Alt 或 Win 中的至少一个修饰键" });
        return;
      }
      void setShortcut(action, event.combo)
        .then(() => setError(null))
        .catch((cause: unknown) => {
          console.error("修改快捷键失败", cause);
          setError({ action, message: typeof cause === "string" ? cause : String(cause) });
        })
        // 成败都以后端列表为准回显：注册/落盘部分失败时界面如实反映
        .finally(load);
    });
    return () => {
      void unlisten.then((off) => off());
      // 离开页面时结束可能仍在进行的录制（不在录制中为空操作）
      void cancelShortcutRecord().catch(() => {});
    };
  }, [load]);

  const startRecording = (action: ShortcutAction) => {
    setError(null);
    setRecording(action);
    void beginShortcutRecord().catch((cause: unknown) => {
      console.error("开始录制失败", cause);
      setError({ action, message: typeof cause === "string" ? cause : String(cause) });
      setRecording(null);
    });
  };

  const stopRecording = useCallback(() => {
    setRecording(null);
    void cancelShortcutRecord().catch(() => {});
  }, []);

  // 录制中点击任意位置即取消（Esc 取消仍由 Rust 钩子经事件返回）。监听在
  // 状态落定后才挂上：触发本次录制的那个 click 不会立刻自取消。
  useEffect(() => {
    if (!recording) return;
    window.addEventListener("mousedown", stopRecording);
    return () => window.removeEventListener("mousedown", stopRecording);
  }, [recording, stopRecording]);

  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-medium text-muted-foreground">快捷键</h2>
      <ScopeGroupCard scope="global">
        {bindings?.map((binding) => (
          <ShortcutRow
            key={binding.action}
            scope="global"
            binding={binding}
            recording={recording === binding.action}
            error={error?.action === binding.action ? error.message : undefined}
            onRecordStart={() => startRecording(binding.action)}
          />
        ))}
      </ScopeGroupCard>
      <ScopeGroupCard scope="clipboard">
        {CLIPBOARD_SCOPE_SHORTCUTS.map(({ combo, title, description }) => (
          <SettingRow
            key={combo}
            title={title}
            tag={SCOPE_META.clipboard.label}
            description={description}
            control={<ComboKbd combo={combo} />}
          />
        ))}
      </ScopeGroupCard>
    </section>
  );
};

/** 容量输入失焦/回车才提交；以引擎 clamp 后的值回显，非法输入恢复原值。
 * 引擎降级（初始化失败）时容量无从生效：输入禁用并如实呈现失效。 */
const ClipboardCapacityRow = () => {
  const [value, setValue] = useState("");
  const [committed, setCommitted] = useState<number | null>(null);
  const [engine, setEngine] = useState<"probing" | "up" | "down">("probing");

  useEffect(() => {
    let active = true;
    void getClipboardCapacity()
      .then((capacity) => {
        if (!active) return;
        setValue(String(capacity));
        setCommitted(capacity);
      })
      .catch((cause) => console.error("读取剪贴板历史容量失败", cause));
    void getClipboardEngineAvailable().then((ok) => {
      if (active) setEngine(ok ? "up" : "down");
    });
    return () => {
      active = false;
    };
  }, []);

  const ready = engine === "up" && committed !== null;

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
      error={engine === "down" ? "剪贴板引擎未启用，历史容量设置不可用" : undefined}
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

/** 锁定超时输入失焦/回车才提交；以后端 clamp 后的值回显，非法输入恢复原值。0 = 永不自动锁定。 */
const VaultLockTimeoutRow = () => {
  const [value, setValue] = useState("");
  const [committed, setCommitted] = useState<number | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let active = true;
    void getVaultLockTimeoutMinutes().then((minutes) => {
      if (!active) return;
      setValue(String(minutes));
      setCommitted(minutes);
      setReady(true);
    });
    return () => {
      active = false;
    };
  }, []);

  const commit = async () => {
    if (!ready) return;
    const parsed = Number(value);
    // 0 = 永不自动锁定；负数或非数字恢复原值，区间钳制交给持久化层
    if (!Number.isFinite(parsed) || parsed < 0) {
      setValue(String(committed));
      return;
    }
    if (parsed === committed) return;
    try {
      await setVaultLockTimeoutMinutes(parsed);
      const applied = await getVaultLockTimeoutMinutes();
      setCommitted(applied);
      setValue(String(applied));
    } catch (error) {
      console.error("修改密码本锁定超时失败", error);
      setValue(String(committed));
    }
  };

  const never = committed === 0;

  return (
    <SettingRow
      title="锁定超时"
      description={
        never
          ? "永不自动锁定，需要时在密码本页手动落锁"
          : "无操作该时长后自动锁定密码本；输入 0 表示永不"
      }
      control={
        <div className="flex items-center gap-2">
          <Input
            type="number"
            min={0}
            value={value}
            disabled={!ready}
            onChange={(e) => setValue(e.target.value)}
            onBlur={() => void commit()}
            onKeyDown={(e) => {
              if (e.key === "Enter") e.currentTarget.blur();
            }}
            className="w-24 text-right"
          />
          <span className="text-sm text-muted-foreground">{never ? "永不" : "分钟"}</span>
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
      <ShortcutsSection />
      <SettingsSection title="剪贴板">
        <ClipboardCapacityRow />
      </SettingsSection>
      <SettingsSection title="划词翻译">
        <DeepSeekApiKeyRow />
        <DeepSeekModelRow />
      </SettingsSection>
      <SettingsSection title="密码本">
        <VaultLockTimeoutRow />
        <SettingRow
          title="主密码"
          description="在密码本页首次使用时设置；主密码丢失后条目数据无法恢复"
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
    void hasTranslateApiKey()
      .then((exists) => {
        if (!active) return;
        setConfigured(exists);
        setReady(true);
      })
      .catch((error) => {
        console.error("读取 DeepSeek API key 状态失败", error);
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
