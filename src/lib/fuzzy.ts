const CONSECUTIVE_BONUS = 8;
const WORD_START_BONUS = 6;
const PREFIX_BONUS = 10;

/** 位置加分随命中下标衰减：命中越早分越高，第 12 列起不再加分。 */
const positionBonus = (index: number): number => Math.max(0, 4 - Math.floor(index / 4));

/**
 * 子序列模糊匹配得分：query 的字符按顺序全部出现在 target 中即命中。
 * 返回越高越靠前的分数；不命中返回 null。空查询视为全匹配、得 0 分。
 *
 * 计分：连续命中、词首/起始命中、查询首字符即目标前缀、命中位置早——
 * 只用于同源短列表排序，不做全局召回。
 */
export const fuzzyScore = (query: string, target: string): number | null => {
  if (query === "") return 0;
  const q = query.toLowerCase();
  const t = target.toLowerCase();
  let score = 0;
  let searchFrom = 0;
  let previousIndex = -2;
  for (let qi = 0; qi < q.length; qi++) {
    const index = t.indexOf(q[qi] as string, searchFrom);
    if (index === -1) return null;
    if (index === previousIndex + 1) score += CONSECUTIVE_BONUS;
    const wordStart = index === 0 || !/[a-z0-9]/.test(t[index - 1] as string);
    if (wordStart) score += WORD_START_BONUS;
    if (qi === 0 && index === 0) score += PREFIX_BONUS;
    score += positionBonus(index);
    previousIndex = index;
    searchFrom = index + 1;
  }
  return score;
};
