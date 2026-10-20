import { AppWindow } from "lucide-react";
import { useEffect, useState } from "react";

import { fetchAppIcon } from "@/lib/apps";

/** 会话内图标缓存：path → PNG 字节；null = 提取失败同样缓存，避免反复发命令。 */
const iconCache = new Map<string, number[] | null>();
const CACHE_LIMIT = 256;

const remember = (path: string, png: number[] | null) => {
  // 插入序即访问序：超限时挤掉最早写入的条目
  if (iconCache.size >= CACHE_LIMIT) {
    const oldest = iconCache.keys().next().value;
    if (oldest !== undefined) iconCache.delete(oldest);
  }
  iconCache.set(path, png);
};

const blobUrlOf = (png: number[]): string =>
  URL.createObjectURL(new Blob([new Uint8Array(png)], { type: "image/png" }));

/** 应用真实图标：异步提取（Rust 侧磁盘缓存），加载中与失败都回退通用窗口图标，
 * 两种形态同尺寸占位，列表不跳动。 */
export const AppIcon = ({ path }: { path: string }) => {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let objectUrl: string | null = null;
    const cached = iconCache.get(path);
    if (cached !== undefined) {
      // 命中（含失败缓存）：不再发命令，失败保持回退图标
      if (cached) {
        objectUrl = blobUrlOf(cached);
        setUrl(objectUrl);
      }
      return () => {
        if (objectUrl) URL.revokeObjectURL(objectUrl);
      };
    }
    setUrl(null);
    void fetchAppIcon(path)
      .then((png) => {
        if (!active) return;
        remember(path, png);
        if (png) {
          objectUrl = blobUrlOf(png);
          setUrl(objectUrl);
        }
      })
      .catch((cause) => console.error("获取应用图标失败", cause));
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [path]);

  if (!url) {
    return <AppWindow className="size-4 shrink-0 text-muted-foreground" aria-hidden />;
  }
  return <img src={url} alt="" className="size-4 shrink-0 object-contain" />;
};
