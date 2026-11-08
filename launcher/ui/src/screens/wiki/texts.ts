// 图鉴页的全部文案
import type { WikiView } from '../../api';

export const WIKI_TEXT = {
  all: '全部',
  newOnly: '本版新增',
  show: '显示',
  search: '查找',
  searchHint: '—',
  noEntries: ['暂无条目', ''] as [string, string],
  noMatch: ['没有符合的条目', '按 ← → 换一种显示'] as [string, string],
  select: '选择',
  adjust: '显示',
  input: '输入',
  clear: '清空',
  done: '完成',
  cancel: '取消',
  back: '返回',
  detail: '详细说明',
  lordSettings: '永夜王设置',
  collapse: '收起',
  scroll: '滚动',
  sourceLabel: '获取方式',
  handNote: '作者补充，非游戏数据',
  noneSearch: '按 Q / E 换到别的分类，有结果的标签是亮的。',
  noneEditing: '按 Enter 结束输入，会跳到第一个有结果的分类。',
  noneFilter: '按 ← → 换一种显示。',
  noneEmpty: '这一类暂时没有条目。',
};

export const newTagText = (version?: string) => `${version ? `${version} ` : ''}新增`;
export const searchMeta = (q: string) => `查找「${q}」`;

type BadState = Exclude<WikiView['state'], 'ok'>;

/** 数据读不出来时的两行字 */
export const WIKI_STATE_TEXT: Record<BadState, [string, string]> = {
  missing: ['图鉴数据缺失', '请重新解压整合包'],
  incompatible: ['图鉴数据需要新版启动器', '更新启动器后再来看'],
  broken: ['图鉴数据损坏', '请重新解压整合包'],
};

/** 查找没有结果；hits 是别的分类里的相符情况，如「护符 3 条」 */
export function notFoundText(query: string, hits: string[], editing: boolean): [string, string] {
  const first = `没有找到「${query}」`;
  if (hits.length > 0) return [first, `${hits.join(' · ')}。${editing ? '按 Enter 结束输入就跳过去' : '按 Q / E 或单击亮着的标签'}`];
  return [first, editing ? '换个字试试，按 Enter 结束输入' : '换个字试试'];
}
