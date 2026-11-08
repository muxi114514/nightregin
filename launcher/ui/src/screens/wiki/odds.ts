// 几率文字：按当前 ChanceSetter 和深夜层数现算；读不到时返回 null，界面回退到 wiki.json 的文字
import type { WikiEntry, WikiOdds } from '../../api';

export const NOTE = '基础几率，未计入防重复与地变保护';

const fmt = (n: number) => String(Math.round(n * 100) / 100);
const sameAll = (v: number[], target: number) => v.every((x) => x === target);

/** 强制深夜时取那一层；否则五层相同写一个数，不同列全部 */
function perDepth(values: number[], level: number | null): string {
  if (level) return `${fmt(values[level - 1])}%`;
  if (sameAll(values, values[0])) return `${fmt(values[0])}%`;
  return `深夜 1～5 层 ${values.map(fmt).join(' / ')}%`;
}

const deepList = (text: string) => text.startsWith('深夜');

export function appearText(e: WikiEntry, o: WikiOdds): string | null {
  const c = e.chance;
  if (!c) return null;
  if (c.table === 'nightlord') {
    const values = o.nightlords?.[c.key];
    if (!values) return null;
    const text = perDepth(values, o.level);
    return deepList(text) ? text : `每次出征 ${text}`;
  }
  if (c.table === 'map') {
    const value = o.maps?.[c.key];
    return value === undefined ? null : `每次出征 ${fmt(value)}%`;
  }
  return null;
}

function everValues(e: WikiEntry, o: WikiOdds): number[] | null {
  const key = e.chance?.key;
  return (key && o.everdark?.[key]) || null;
}

/** 当前层（没强制时是所有层）都是这个几率吗 */
function everIs(values: number[], level: number | null, target: number): boolean {
  return level ? values[level - 1] === target : sameAll(values, target);
}

/** 必定以永夜形态出现：头像换成永夜版 */
export function showsEver(e: WikiEntry, o: WikiOdds): boolean {
  const values = everValues(e, o);
  return e.lord !== undefined && !!values && everIs(values, o.level, 100);
}

/** 只有能成为永夜王的夜王才有这一行 */
export function everText(e: WikiEntry, o: WikiOdds): string | null {
  if (e.lord === undefined) return null;
  const values = everValues(e, o);
  if (!values) return '按游戏原规则出现';
  if (everIs(values, o.level, 100)) return `以永夜王形态出现：${e.everName ?? e.name}`;
  if (everIs(values, o.level, 0)) return '以普通形态出现';
  return `${perDepth(values, o.level)} 以永夜王形态出现`;
}
