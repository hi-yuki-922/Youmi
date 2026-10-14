import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Eye, EyeOff, KeyRound, Lock, Pencil, Plus, Search, Trash2 } from "lucide-react";
import { useDeferredValue, useEffect, useRef, useState, type ReactNode } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Textarea } from "@/components/ui/Textarea";
import {
  copyVaultField,
  createVaultEntry,
  deleteVaultEntry,
  getVaultStatus,
  lockVault,
  onVaultLocked,
  queryVaultEntries,
  setupVault,
  unlockVault,
  updateVaultEntry,
  type VaultCopyField,
  type VaultEntry,
  type VaultEntryInput,
} from "@/lib/vault";

const MIN_MASTER_PASSWORD_LEN = 8;

/** tauri command 的错误就是 Rust 侧的 Display 消息；取不到时退回兜底文案。 */
const errorMessage = (error: unknown, fallback: string): string =>
  typeof error === "string" && error.trim() ? error : fallback;

const PasswordBookPage = () => {
  const queryClient = useQueryClient();
  const statusQuery = useQuery({ queryKey: ["vault-status"], queryFn: getVaultStatus });

  // 超时自动锁定由 Rust 哨兵线程发事件：失效状态查询，页面切回解锁界面
  useEffect(() => {
    const unlisten = onVaultLocked(() => {
      void queryClient.invalidateQueries({ queryKey: ["vault-status"] });
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, [queryClient]);

  if (statusQuery.isPending) {
    return <p className="p-6 text-sm text-muted-foreground">加载中…</p>;
  }
  if (statusQuery.isError) {
    return (
      <p className="p-6 text-sm text-destructive">
        密码本状态读取失败：{errorMessage(statusQuery.error, "未知错误")}
      </p>
    );
  }

  const refreshSession = () => {
    void queryClient.invalidateQueries({ queryKey: ["vault-status"] });
    void queryClient.invalidateQueries({ queryKey: ["vault-entries"] });
  };

  if (!statusQuery.data.initialized) {
    return <SetupView onReady={refreshSession} />;
  }
  if (!statusQuery.data.unlocked) {
    return <UnlockView onUnlocked={refreshSession} />;
  }
  return <VaultView onLocked={refreshSession} />;
};

const AuthPanel = ({
  title,
  description,
  error,
  onSubmit,
  children,
}: {
  title: string;
  description: string;
  error?: string | null;
  onSubmit: () => void;
  children: ReactNode;
}) => (
  <div className="flex h-full items-center justify-center p-6">
    <form
      className="flex w-80 flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <div className="flex flex-col items-center gap-1.5 py-2">
        <KeyRound className="size-8 text-muted-foreground" />
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="text-center text-xs text-muted-foreground">{description}</p>
      </div>
      {children}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </form>
  </div>
);

/** 首次使用：设置主密码（加密密钥由此派生，没有找回手段）。 */
const SetupView = ({ onReady }: { onReady: () => void }) => {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const submit = async () => {
    if (submitting) return;
    setError(null);
    if (password.length < MIN_MASTER_PASSWORD_LEN) {
      setError(`主密码至少需要 ${MIN_MASTER_PASSWORD_LEN} 位`);
      return;
    }
    if (password !== confirm) {
      setError("两次输入的主密码不一致");
      return;
    }
    setSubmitting(true);
    try {
      await setupVault(password);
      onReady();
    } catch (e) {
      setError(errorMessage(e, "设置主密码失败"));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <AuthPanel
      title="设置主密码"
      description="主密码用于加密全部条目，丢失后数据无法恢复——请牢记"
      error={error}
      onSubmit={() => void submit()}
    >
      <Input
        type="password"
        autoComplete="new-password"
        placeholder="主密码（至少 8 位）"
        value={password}
        autoFocus
        onChange={(e) => setPassword(e.target.value)}
      />
      <Input
        type="password"
        autoComplete="new-password"
        placeholder="确认主密码"
        value={confirm}
        onChange={(e) => setConfirm(e.target.value)}
      />
      <Button type="submit" disabled={submitting || !password || !confirm}>
        开始使用
      </Button>
    </AuthPanel>
  );
};

/** 已初始化：主密码解锁；错误密码由后端拒绝，消息原样展示。 */
const UnlockView = ({ onUnlocked }: { onUnlocked: () => void }) => {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const submit = async () => {
    if (submitting || !password) return;
    setError(null);
    setSubmitting(true);
    try {
      await unlockVault(password);
      onUnlocked();
    } catch (e) {
      setError(errorMessage(e, "解锁失败"));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <AuthPanel
      title="密码本已锁定"
      description="输入主密码解锁"
      error={error}
      onSubmit={() => void submit()}
    >
      <Input
        type="password"
        autoComplete="current-password"
        placeholder="主密码"
        value={password}
        autoFocus
        onChange={(e) => setPassword(e.target.value)}
      />
      <Button type="submit" disabled={submitting || !password}>
        解锁
      </Button>
    </AuthPanel>
  );
};

/** 复制提示的展示时长。 */
const COPIED_HINT_MS = 3000;

/** 解锁后的主界面：搜索 + 条目列表 + 新建/锁定，条目编辑走模态表单。 */
const VaultView = ({ onLocked }: { onLocked: () => void }) => {
  const [keyword, setKeyword] = useState("");
  const deferredKeyword = useDeferredValue(keyword);
  /** null = 关闭；"new" = 新建；VaultEntry = 编辑该条目。 */
  const [editing, setEditing] = useState<"new" | VaultEntry | null>(null);
  const [copiedHint, setCopiedHint] = useState<string | null>(null);
  const hintTimerRef = useRef<number | null>(null);

  const queryClient = useQueryClient();
  const entriesQuery = useQuery({
    queryKey: ["vault-entries", deferredKeyword],
    queryFn: () => queryVaultEntries(deferredKeyword || undefined),
    placeholderData: (previous) => previous, // 搜索输入时列表不闪空
  });
  const refreshEntries = () => {
    void queryClient.invalidateQueries({ queryKey: ["vault-entries"] });
  };

  const lockMutation = useMutation({
    mutationFn: lockVault,
    onSuccess: onLocked,
    onError: (error) => console.error("落锁失败", error),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => deleteVaultEntry(id),
    onSuccess: refreshEntries,
    onError: (error) => console.error("删除条目失败", error),
  });

  const showCopiedHint = (text: string) => {
    setCopiedHint(text);
    if (hintTimerRef.current !== null) window.clearTimeout(hintTimerRef.current);
    hintTimerRef.current = window.setTimeout(() => setCopiedHint(null), COPIED_HINT_MS);
  };

  const copy = async (entry: VaultEntry, field: VaultCopyField) => {
    try {
      await copyVaultField(entry.id, field);
      showCopiedHint(field === "password" ? "已复制密码，30 秒后自动清除" : "已复制用户名");
    } catch (error) {
      console.error("复制失败", error);
    }
  };

  useEffect(
    () => () => {
      if (hintTimerRef.current !== null) window.clearTimeout(hintTimerRef.current);
    },
    [],
  );

  const entries = entriesQuery.data ?? [];

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center gap-2 border-b px-4 py-3">
        <div className="relative w-72 max-w-full">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            placeholder="搜索标题或用户名…"
            className="h-8 pl-8"
          />
        </div>
        {copiedHint && <span className="text-xs text-muted-foreground">{copiedHint}</span>}
        <div className="ml-auto flex shrink-0 items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={lockMutation.isPending}
            onClick={() => lockMutation.mutate()}
          >
            <Lock />
            锁定
          </Button>
          <Button size="sm" onClick={() => setEditing("new")}>
            <Plus />
            新条目
          </Button>
        </div>
      </div>

      {entriesQuery.isPending ? (
        <p className="p-4 text-sm text-muted-foreground">加载中…</p>
      ) : entries.length === 0 ? (
        <p className="p-4 text-sm text-muted-foreground">
          {deferredKeyword
            ? `没有匹配「${deferredKeyword}」的条目`
            : "暂无条目，点击「新条目」添加"}
        </p>
      ) : (
        <ul className="flex-1 overflow-y-auto">
          {entries.map((entry) => (
            <EntryRow
              key={entry.id}
              entry={entry}
              onCopy={copy}
              onEdit={() => setEditing(entry)}
              onDelete={() => deleteMutation.mutate(entry.id)}
            />
          ))}
        </ul>
      )}

      {editing && (
        <EntryDialog
          entry={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            refreshEntries();
          }}
        />
      )}
    </div>
  );
};

/** 行内悬停显隐的图标按钮（编辑/删除共用）。 */
const rowIconButtonClass =
  "rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-accent hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100";

const EntryRow = ({
  entry,
  onCopy,
  onEdit,
  onDelete,
}: {
  entry: VaultEntry;
  onCopy: (entry: VaultEntry, field: VaultCopyField) => void;
  onEdit: () => void;
  onDelete: () => void;
}) => (
  <li className="group flex items-center gap-3 border-b px-4 py-2.5 last:border-b-0 hover:bg-accent/50">
    <div className="min-w-0 flex-1">
      <p className="truncate text-sm font-medium">{entry.title}</p>
      <p className="truncate text-xs text-muted-foreground">
        {entry.username}
        {entry.url && <span className="ml-2">{entry.url}</span>}
      </p>
    </div>
    <div className="flex shrink-0 items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        disabled={!entry.username}
        onClick={() => onCopy(entry, "username")}
      >
        复制用户名
      </Button>
      <Button
        variant="ghost"
        size="sm"
        disabled={!entry.password}
        onClick={() => onCopy(entry, "password")}
      >
        复制密码
      </Button>
      <button type="button" onClick={onEdit} aria-label="编辑条目" className={rowIconButtonClass}>
        <Pencil className="size-4" />
      </button>
      <ConfirmDeleteButton onDelete={onDelete} />
    </div>
  </li>
);

/** 删除凭据条目：两段式——首击进入确认态，限时内再击才执行；Esc、失焦或超时退回。 */
const CONFIRM_DELETE_TIMEOUT_MS = 3000;
/** 进入确认态后忽略点击的窗口：原地快速双击不该等于确认。 */
const CONFIRM_DELETE_GRACE_MS = 400;

const ConfirmDeleteButton = ({ onDelete }: { onDelete: () => void }) => {
  const [armed, setArmed] = useState(false);
  const armedAtRef = useRef(0);
  const disarmTimerRef = useRef<number | null>(null);

  const disarm = () => {
    if (disarmTimerRef.current !== null) window.clearTimeout(disarmTimerRef.current);
    disarmTimerRef.current = null;
    setArmed(false);
  };

  const arm = () => {
    armedAtRef.current = Date.now();
    setArmed(true);
    disarmTimerRef.current = window.setTimeout(disarm, CONFIRM_DELETE_TIMEOUT_MS);
  };

  useEffect(() => () => disarm(), []);

  useEffect(() => {
    if (!armed) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") disarm();
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("blur", disarm);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("blur", disarm);
    };
  }, [armed]);

  if (!armed) {
    return (
      <button type="button" onClick={arm} aria-label="删除条目" className={rowIconButtonClass}>
        <Trash2 className="size-4" />
      </button>
    );
  }
  return (
    <Button
      variant="destructive"
      size="sm"
      autoFocus
      onClick={() => {
        if (Date.now() - armedAtRef.current < CONFIRM_DELETE_GRACE_MS) return;
        disarm();
        onDelete();
      }}
    >
      <Trash2 />
      确认删除
    </Button>
  );
};

const EntryField = ({ label, children }: { label: string; children: ReactNode }) => (
  <label className="flex flex-col gap-1">
    <span className="text-xs text-muted-foreground">{label}</span>
    {children}
  </label>
);

/** 新建/编辑条目的模态表单：背景点击、Esc、取消按钮都走 onClose。 */
const EntryDialog = ({
  entry,
  onClose,
  onSaved,
}: {
  entry: VaultEntry | null;
  onClose: () => void;
  onSaved: () => void;
}) => {
  const [title, setTitle] = useState(entry?.title ?? "");
  const [username, setUsername] = useState(entry?.username ?? "");
  const [password, setPassword] = useState(entry?.password ?? "");
  const [url, setUrl] = useState(entry?.url ?? "");
  const [notes, setNotes] = useState(entry?.notes ?? "");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const save = async () => {
    if (saving) return;
    setError(null);
    if (!title.trim()) {
      setError("标题不能为空");
      return;
    }
    setSaving(true);
    const input: VaultEntryInput = { title, username, password, url, notes };
    try {
      if (entry) {
        await updateVaultEntry(entry.id, input);
      } else {
        await createVaultEntry(input);
      }
      onSaved();
    } catch (e) {
      setError(errorMessage(e, "保存失败"));
      setSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-6"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <form
        className="flex w-96 max-w-full flex-col gap-3 rounded-lg border bg-card p-5 shadow-lg"
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <h2 className="text-base font-semibold">{entry ? "编辑条目" : "新建条目"}</h2>
        <EntryField label="标题">
          <Input
            value={title}
            placeholder="如 GitHub"
            autoFocus
            onChange={(e) => setTitle(e.target.value)}
          />
        </EntryField>
        <EntryField label="用户名">
          <Input value={username} onChange={(e) => setUsername(e.target.value)} />
        </EntryField>
        <EntryField label="密码">
          <div className="relative">
            <Input
              type={showPassword ? "text" : "password"}
              autoComplete="new-password"
              value={password}
              className="pr-9"
              onChange={(e) => setPassword(e.target.value)}
            />
            <button
              type="button"
              aria-label={showPassword ? "隐藏密码" : "显示密码"}
              onClick={() => setShowPassword((v) => !v)}
              className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground hover:text-foreground"
            >
              {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            </button>
          </div>
        </EntryField>
        <EntryField label="URL">
          <Input value={url} placeholder="https://…" onChange={(e) => setUrl(e.target.value)} />
        </EntryField>
        <EntryField label="备注">
          <Textarea value={notes} className="min-h-16" onChange={(e) => setNotes(e.target.value)} />
        </EntryField>
        {error && <p className="text-xs text-destructive">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" size="sm" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" size="sm" disabled={saving}>
            保存
          </Button>
        </div>
      </form>
    </div>
  );
};

export const Route = createFileRoute("/_main/password-book")({
  component: PasswordBookPage,
});
