import { useEffect, useState, type Dispatch, type SetStateAction } from "react";

/**
 * 工具页状态持久化：localStorage 承载（工具内容无敏感要求），键按工具划分
 * （`tools.<tool>.<field>`），挂载时恢复、变更即写回。路由切换卸载/重挂与
 * 应用重启对页面组件等价——都走同一份存档。
 */

export type ToolStateStorage = Pick<Storage, "getItem" | "setItem">;

const KEY_PREFIX = "tools.";

const storageKey = (key: string): string => KEY_PREFIX + key;

/** 序列化保存：空状态同样落盘——清空后重启不得复活旧内容。 */
export const saveToolState = (storage: ToolStateStorage, key: string, state: unknown): void => {
  try {
    storage.setItem(storageKey(key), JSON.stringify(state));
  } catch (error) {
    console.error(`保存工具状态失败（${key}）`, error);
  }
};

/**
 * 恢复：无存档、JSON 损坏或形状不符（restore 返回 undefined）一律回退默认值。
 * 存档是外部数据，形状可能因版本演进或损坏而与当前类型不符，不做信任假设。
 */
export const loadToolState = <T>(
  storage: ToolStateStorage,
  key: string,
  fallback: T,
  restore: (value: unknown) => T | undefined,
): T => {
  try {
    const raw = storage.getItem(storageKey(key));
    if (raw === null) return fallback;
    // restore 的 undefined 表示形状不符；null 可能是合法状态（可空错误态），不能并入
    const restored = restore(JSON.parse(raw));
    return restored === undefined ? fallback : restored;
  } catch (error) {
    console.error(`恢复工具状态失败（${key}）`, error);
    return fallback;
  }
};

/**
 * 持久化的 useState：初值恢复自 localStorage，每次变更写回。
 * restore 校验存档形状（见 loadToolState）。
 */
export const usePersistedToolState = <T>(
  key: string,
  fallback: T,
  restore: (value: unknown) => T | undefined,
): [T, Dispatch<SetStateAction<T>>] => {
  const [state, setState] = useState<T>(() =>
    loadToolState(window.localStorage, key, fallback, restore),
  );
  useEffect(() => {
    saveToolState(window.localStorage, key, state);
  }, [key, state]);
  return [state, setState];
};

/** 字符串字段恢复。 */
export const restoreString = (value: unknown): string | undefined =>
  typeof value === "string" ? value : undefined;

/** 可空字符串字段恢复（如转换错误态的「无错误」null）。 */
export const restoreNullableString = (value: unknown): string | null | undefined =>
  value === null || typeof value === "string" ? value : undefined;

/** 枚举字段恢复：存档值命中候选集之一才接受（如视图模式、子 tab、语言）。 */
export const restoreOneOf =
  <T>(values: readonly T[]) =>
  (value: unknown): T | undefined =>
    values.some((candidate) => candidate === value) ? (value as T) : undefined;

/**
 * 字符串数组字段恢复：逐项过滤进候选集并去重保序（如目标币种行）。
 * 空结果或形状不符回退 undefined 由调用方给默认值——列表类状态不允许存档
 * 把页面打成空壳。
 */
export const restoreStringArray =
  (values: readonly string[]) =>
  (value: unknown): string[] | undefined => {
    if (!Array.isArray(value)) return undefined;
    const cleaned = value.filter(
      (entry): entry is string =>
        typeof entry === "string" && values.some((candidate) => candidate === entry),
    );
    return [...new Set(cleaned)];
  };
