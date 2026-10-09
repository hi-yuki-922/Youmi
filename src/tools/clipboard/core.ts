const pad2 = (n: number) => String(n).padStart(2, "0");

/** 本地时区的 月-日 时:分（无秒、无年份）。 */
function formatMonthDayTime(d: Date): string {
  return `${pad2(d.getMonth() + 1)}-${pad2(d.getDate())} ${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

/**
 * 条目时间显示：今天 → "HH:mm"；今年 → "MM-DD HH:mm"；更早 → "YYYY-MM-DD"。
 * `now` 可注入（测试用），默认当前时间。
 */
export function formatEntryTime(timestampMs: number, now: Date = new Date()): string {
  const date = new Date(timestampMs);
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
  }
  if (date.getFullYear() === now.getFullYear()) {
    return formatMonthDayTime(date);
  }
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
}
