import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Image as ImageIcon, Search, Trash2 } from "lucide-react";
import { useDeferredValue, useEffect, useRef, useState } from "react";

import { Input } from "@/components/ui/input";
import { SegmentedTabs } from "@/components/ui/segmented-tabs";
import {
  deleteClipboardEntry,
  getClipboardImage,
  queryClipboardEntries,
  type ClipboardEntry,
  type ClipboardKind,
} from "@/lib/clipboard";
import { formatEntryTime } from "@/tools/clipboard/core";

export const Route = createFileRoute("/overlay/clipboard")({
  component: ClipboardOverlay,
});

type KindFilter = "all" | ClipboardKind;

/** 浏览列表条数上限：与默认容量对齐；更大容量 + 无过滤浏览留待分页。 */
const LIST_QUERY_LIMIT = 1000;

const KIND_FILTER_OPTIONS: ReadonlyArray<{ id: KindFilter; label: string }> = [
  { id: "all", label: "全部" },
  { id: "text", label: "文本" },
  { id: "image", label: "图片" },
];

function ClipboardOverlay() {
  const [keyword, setKeyword] = useState("");
  const deferredKeyword = useDeferredValue(keyword);
  const [kind, setKind] = useState<KindFilter>("all");
  const searchRef = useRef<HTMLInputElement>(null);

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

  // 浮层 hide 而非销毁：重新呼起时找回搜索框焦点，并刷新历史
  // （WebView2 显示/隐藏窗口不派发 visibilitychange——实测——tanstack-query 的
  // refetchOnWindowFocus 在这里永不触发，必须手动 refetch）。
  const refetch = entriesQuery.refetch;
  useEffect(() => {
    searchRef.current?.focus();
    const onWindowFocus = () => {
      searchRef.current?.focus();
      void refetch();
    };
    window.addEventListener("focus", onWindowFocus);
    return () => window.removeEventListener("focus", onWindowFocus);
  }, [refetch]);

  const queryClient = useQueryClient();
  const deleteMutation = useMutation({
    mutationFn: (id: number) => deleteClipboardEntry(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["clipboard-entries"] });
    },
    onError: (error) => {
      console.error("删除剪贴板条目失败", error);
    },
  });

  const entries = entriesQuery.data ?? [];

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-sm">
      <div className="flex items-center gap-2 border-b px-3 py-2.5">
        <div className="relative min-w-0 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={searchRef}
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
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
      </div>

      {entriesQuery.isPending ? (
        <p className="p-4 text-muted-foreground">加载中…</p>
      ) : entries.length === 0 ? (
        <EmptyState keyword={deferredKeyword} kind={kind} />
      ) : (
        <ul className="flex-1 overflow-y-auto">
          {entries.map((entry) => (
            <EntryRow
              key={entry.id}
              entry={entry}
              onDelete={() => deleteMutation.mutate(entry.id)}
            />
          ))}
        </ul>
      )}
    </div>
  );
}

function EmptyState({ keyword, kind }: { keyword: string; kind: KindFilter }) {
  let message = "暂无剪贴板历史，复制的内容会自动出现在这里";
  if (keyword) {
    message = `没有匹配「${keyword}」的条目`;
  } else if (kind === "text") {
    message = "暂无文本条目";
  } else if (kind === "image") {
    message = "暂无图片条目";
  }
  return <p className="p-4 text-muted-foreground">{message}</p>;
}

function EntryRow({ entry, onDelete }: { entry: ClipboardEntry; onDelete: () => void }) {
  return (
    <li className="group flex items-center gap-3 border-b px-4 py-2.5 last:border-b-0 hover:bg-accent/50">
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
        aria-label="删除该条目"
        className="shrink-0 rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-accent hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
      >
        <Trash2 className="size-4" />
      </button>
    </li>
  );
}

/** 图片缩略图：updatedAt 进 queryKey——id 可能被 SQLite 复用，同 id 同 updatedAt 才视为同图。 */
function ImageThumb({ id, updatedAt }: { id: number; updatedAt: number }) {
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
}
