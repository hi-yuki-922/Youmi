import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Image as ImageIcon, Search, Trash2 } from "lucide-react";
import { useDeferredValue, useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Kbd } from "@/components/ui/Kbd";
import { SegmentedTabs } from "@/components/ui/SegmentedTabs";
import {
  clearClipboardHistory,
  deleteClipboardEntry,
  getClipboardImage,
  pastebackClipboardEntry,
  queryClipboardEntries,
  type ClipboardEntry,
  type ClipboardKind,
} from "@/lib/clipboard";
import { formatEntryTime } from "@/tools/clipboard/core";

type KindFilter = "all" | ClipboardKind;

/** 浏览列表条数上限：与默认容量对齐；更大容量 + 无过滤浏览留待分页。 */
const LIST_QUERY_LIMIT = 1000;

/** 确认态自动退回的时限：足够读清提示，又不至于一直占着危险按钮。 */
const CLEAR_CONFIRM_TIMEOUT_MS = 3000;

/** 进入确认态后忽略点击的窗口：原地快速双击不该等于确认。 */
const CLEAR_CONFIRM_GRACE_MS = 400;

const KIND_FILTER_OPTIONS: ReadonlyArray<{ id: KindFilter; label: string }> = [
  { id: "all", label: "全部" },
  { id: "text", label: "文本" },
  { id: "image", label: "图片" },
];

/** 焦点移到列表中相邻的行（clamp 在首尾）。 */
const focusSiblingRow = (row: HTMLElement, offset: 1 | -1) => {
  const rows = row.parentElement?.children;
  if (!rows) return;
  const index = Array.prototype.indexOf.call(rows, row) + offset;
  if (index < 0 || index >= rows.length) return;
  if (rows[index] instanceof HTMLElement) rows[index].focus();
};

const ClipboardOverlay = () => {
  const [keyword, setKeyword] = useState("");
  const deferredKeyword = useDeferredValue(keyword);
  const [kind, setKind] = useState<KindFilter>("all");
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const entriesQuery = useQuery({
    queryKey: ["clipboard-entries", { keyword: deferredKeyword, kind }],
    queryFn: () =>
      queryClipboardEntries({
        keyword: deferredKeyword || undefined,
        kind: kind === "all" ? undefined : kind,
        limit: LIST_QUERY_LIMIT,
      }),
    placeholderData: (previous) => previous, // 搜索输入时列表不闪空
  });

  const queryClient = useQueryClient();

  // 浮层 hide 而非销毁：重新呼起时找回搜索框焦点，并刷新历史
  // （WebView2 显示/隐藏窗口不派发 visibilitychange——实测——tanstack-query 的
  // refetchOnWindowFocus 在这里永不触发，必须手动刷新；列表各视图与清空入口
  // 的显隐查询同前缀，一并 refetch，隐藏期捕获的新条目才都显现）。
  useEffect(() => {
    searchRef.current?.focus();
    const onWindowFocus = () => {
      searchRef.current?.focus();
      void queryClient.refetchQueries({ queryKey: ["clipboard-entries"] });
    };
    window.addEventListener("focus", onWindowFocus);
    return () => window.removeEventListener("focus", onWindowFocus);
  }, [queryClient]);

  const deleteMutation = useMutation({
    mutationFn: (id: number) => deleteClipboardEntry(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["clipboard-entries"] });
    },
    onError: (error) => {
      console.error("删除剪贴板条目失败", error);
    },
  });

  const clearMutation = useMutation({
    mutationFn: clearClipboardHistory,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["clipboard-entries"] });
      searchRef.current?.focus();
    },
    onError: (error) => {
      console.error("清空剪贴板历史失败", error);
    },
  });

  // 回帖不改变历史（Rust 自家写入通道 + 完成后恢复原剪贴板），无需 invalidate；
  // 浮层由 Rust 侧在回帖流程中隐藏。copied-only 属正常降级，不打扰用户。
  const pastebackMutation = useMutation({
    mutationFn: (id: number) => pastebackClipboardEntry(id),
    onError: (error) => {
      console.error("回帖失败", error);
    },
  });

  const pasteback = (id: number) => {
    if (!pastebackMutation.isPending) pastebackMutation.mutate(id);
  };

  const entries = entriesQuery.data ?? [];

  // 清空作用于全库而非当前过滤结果：入口显隐跟随未过滤历史是否非空
  const hasAnyQuery = useQuery({
    queryKey: ["clipboard-entries", "any"],
    queryFn: () => queryClipboardEntries({ limit: 1 }),
    placeholderData: (previous) => previous,
  });
  const hasAny = (hasAnyQuery.data?.length ?? 0) > 0;

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-sm">
      <div className="flex items-center gap-2 border-b px-3 py-2.5">
        <div className="relative min-w-0 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={searchRef}
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            onKeyDown={(e) => {
              // 方向键下 = 从搜索框进入列表选择
              if (e.key === "ArrowDown") {
                e.preventDefault();
                const first = listRef.current?.firstElementChild;
                if (first instanceof HTMLElement) first.focus();
              }
            }}
            placeholder="搜索剪贴板历史…"
            className="h-8 pl-8"
          />
        </div>
        <SegmentedTabs
          aria-label="按类型过滤"
          options={KIND_FILTER_OPTIONS}
          value={kind}
          onChange={setKind}
          className="shrink-0"
        />
        {hasAny && (
          <ClearHistoryButton
            onClear={() => clearMutation.mutate()}
            pending={clearMutation.isPending}
          />
        )}
      </div>

      {entriesQuery.isPending ? (
        <p className="p-4 text-muted-foreground">加载中…</p>
      ) : entries.length === 0 ? (
        <EmptyState keyword={deferredKeyword} kind={kind} />
      ) : (
        <ul ref={listRef} className="flex-1 overflow-y-auto">
          {entries.map((entry) => (
            <EntryRow
              key={entry.id}
              entry={entry}
              onPaste={() => pasteback(entry.id)}
              onDelete={() => deleteMutation.mutate(entry.id)}
            />
          ))}
        </ul>
      )}

      <p className="flex items-center gap-1.5 border-t px-4 py-1.5 text-xs text-muted-foreground">
        <Kbd>↑</Kbd>
        <Kbd>↓</Kbd>
        <span>选择</span>
        <span>·</span>
        <Kbd>⏎</Kbd>
        <span>或双击回帖到之前的窗口</span>
      </p>
    </div>
  );
};

const EmptyState = ({ keyword, kind }: { keyword: string; kind: KindFilter }) => {
  let message = "暂无剪贴板历史，复制的内容会自动出现在这里";
  if (keyword) {
    message = `没有匹配「${keyword}」的条目`;
  } else if (kind === "text") {
    message = "暂无文本条目";
  } else if (kind === "image") {
    message = "暂无图片条目";
  }
  return <p className="p-4 text-muted-foreground">{message}</p>;
};

/** 清空全部历史：两段式——首击进入确认态，限时内再击才执行；Esc、失焦或超时退回。 */
const ClearHistoryButton = ({ onClear, pending }: { onClear: () => void; pending: boolean }) => {
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
    disarmTimerRef.current = window.setTimeout(disarm, CLEAR_CONFIRM_TIMEOUT_MS);
  };

  useEffect(() => () => disarm(), []);

  // 确认态下按 Esc 退回，不依赖焦点仍在按钮上；窗口失焦也退回——浮层 hide
  // 而非销毁，确认态会跨隐藏存活，重新呼起不该停留在"一击即清"的状态。
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
      <Button
        variant="ghost"
        size="icon"
        aria-label="清空全部历史"
        title="清空全部历史"
        onClick={arm}
        className="size-8 shrink-0 text-muted-foreground"
      >
        <Trash2 />
      </Button>
    );
  }
  return (
    <Button
      variant="destructive"
      size="sm"
      className="shrink-0"
      autoFocus
      disabled={pending}
      onClick={() => {
        if (Date.now() - armedAtRef.current < CLEAR_CONFIRM_GRACE_MS) return;
        disarm();
        onClear();
      }}
    >
      <Trash2 />
      确认清空
    </Button>
  );
};

const EntryRow = ({
  entry,
  onPaste,
  onDelete,
}: {
  entry: ClipboardEntry;
  onPaste: () => void;
  onDelete: () => void;
}) => {
  return (
    <li
      tabIndex={0}
      aria-label={entry.kind === "image" ? "图片条目，回车或双击回帖" : `文本条目，回车或双击回帖`}
      onDoubleClick={onPaste}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          // 焦点在行内删除按钮上时，Enter 仍归按钮（删除），不触发回帖
          if (e.target !== e.currentTarget) return;
          e.preventDefault();
          onPaste();
        } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          e.preventDefault();
          focusSiblingRow(e.currentTarget, e.key === "ArrowDown" ? 1 : -1);
        }
      }}
      className="group flex cursor-default items-center gap-3 border-b px-4 py-2.5 last:border-b-0 focus:bg-accent/70 focus:outline-none hover:bg-accent/50"
    >
      <div className="min-w-0 flex-1">
        {entry.kind === "image" ? (
          <ImageThumb id={entry.id} updatedAt={entry.updatedAt} />
        ) : (
          <p className="line-clamp-3 break-all whitespace-pre-wrap">{entry.text}</p>
        )}
      </div>
      <span className="shrink-0 text-xs text-muted-foreground">
        {formatEntryTime(entry.updatedAt)}
      </span>
      <button
        type="button"
        onClick={onDelete}
        onDoubleClick={(e) => e.stopPropagation()}
        aria-label="删除该条目"
        className="shrink-0 rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-accent hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
      >
        <Trash2 className="size-4" />
      </button>
    </li>
  );
};

/** 图片缩略图：updatedAt 进 queryKey——id 可能被 SQLite 复用，同 id 同 updatedAt 才视为同图。 */
const ImageThumb = ({ id, updatedAt }: { id: number; updatedAt: number }) => {
  const { data } = useQuery({
    queryKey: ["clipboard-image", id, updatedAt],
    queryFn: () => getClipboardImage(id),
    staleTime: Infinity,
  });

  if (!data) {
    return (
      <div className="flex h-16 w-24 items-center justify-center rounded bg-muted text-muted-foreground">
        <ImageIcon className="size-5" />
      </div>
    );
  }
  return (
    <img
      src={`data:image/png;base64,${data}`}
      alt="剪贴板图片"
      className="max-h-16 max-w-40 rounded border object-contain"
    />
  );
};

export const Route = createFileRoute("/overlay/clipboard")({
  component: ClipboardOverlay,
});
