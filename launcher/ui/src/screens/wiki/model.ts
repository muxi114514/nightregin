// 图鉴的纯状态：分类、筛选、查找、各分类记住的选中项；不碰 DOM
import type { WikiCategory, WikiData, WikiEntry } from '../../api';
import { sanitize } from './sanitize';
import { WIKI_TEXT } from './texts';

export type Layout = 'grid' | 'thumbs' | 'list';

const LAYOUTS: readonly string[] = ['grid', 'thumbs', 'list'];

/** 不认识的版式按单行列表显示 */
export const layoutOf = (c: WikiCategory): Layout => (LAYOUTS.includes(c.layout ?? '') ? (c.layout as Layout) : 'list');

const entriesOf = (c: WikiCategory): WikiEntry[] => c.entries ?? [];

export class WikiModel {
  readonly cats: WikiCategory[];
  readonly version: string | undefined;
  catIndex = 0;
  query = '';
  private readonly newIds: Set<string>;
  private readonly filters = new Map<string, number>();
  private readonly selected = new Map<string, string>();
  /** 查找用的文本，每条只拼一次 */
  private readonly hay = new Map<string, string>();

  constructor(data: WikiData, newIds: string[]) {
    const clean = sanitize(data);
    this.version = clean.version;
    this.newIds = new Set(newIds);
    this.cats = clean.categories
      .map((c, i) => ({ c, i }))
      .sort((a, b) => (a.c.order ?? 0) - (b.c.order ?? 0) || a.i - b.i)
      .map(({ c }) => c);
    for (const c of this.cats) {
      for (const e of entriesOf(c)) {
        this.hay.set(e.id, [e.name, e.sub ?? '', ...(e.stats ?? []).map((s) => s.v)].join(' ').toLowerCase());
      }
    }
  }

  cat(): WikiCategory {
    return this.cats[this.catIndex];
  }

  setCat(i: number): void {
    const n = this.cats.length;
    this.catIndex = ((i % n) + n) % n;
  }

  isNew(id: string): boolean {
    return this.newIds.has(id);
  }

  hasNew(c: WikiCategory): boolean {
    return entriesOf(c).some((e) => this.isNew(e.id));
  }

  /** 全部 / 本版新增（有才列）/ 具体类别（不止一项才列） */
  options(c: WikiCategory = this.cat()): string[] {
    const own = c.filters ?? [];
    return [WIKI_TEXT.all, ...(this.hasNew(c) ? [WIKI_TEXT.newOnly] : []), ...(own.length > 1 ? own : [])];
  }

  filterIndex(c: WikiCategory = this.cat()): number {
    return this.filters.get(c.id) ?? 0;
  }

  /** 到边界停住，没变返回 false */
  setFilter(delta: -1 | 1): boolean {
    const c = this.cat();
    const now = this.filterIndex(c);
    const next = Math.max(0, Math.min(this.options(c).length - 1, now + delta));
    if (next === now) return false;
    this.filters.set(c.id, next);
    return true;
  }

  /** 筛选 + 查找后的条目；useFilter 为 false 时只看查找，用来数别的分类有几条相符 */
  list(c: WikiCategory = this.cat(), useFilter = true): WikiEntry[] {
    const opts = this.options(c);
    const opt = useFilter ? (opts[this.filterIndex(c)] ?? WIKI_TEXT.all) : WIKI_TEXT.all;
    const q = this.query.trim().toLowerCase();
    return entriesOf(c).filter((e) => this.passes(e, opt) && (!q || (this.hay.get(e.id) ?? '').includes(q)));
  }

  private passes(e: WikiEntry, opt: string): boolean {
    if (opt === WIKI_TEXT.all) return true;
    if (opt === WIKI_TEXT.newOnly) return this.isNew(e.id);
    return e.filters?.includes(opt) ?? false;
  }

  /** 第 i 个分类里忽略筛选的相符条数 */
  matchCount(i: number): number {
    return this.list(this.cats[i], false).length;
  }

  current(): WikiEntry | undefined {
    const list = this.list();
    const id = this.selected.get(this.cat().id);
    return list.find((e) => e.id === id) ?? list[0];
  }

  select(id: string): void {
    this.selected.set(this.cat().id, id);
  }

  /** 跳到某条：换分类，清掉筛选和查找 */
  focusEntry(id: string): boolean {
    const i = this.cats.findIndex((c) => entriesOf(c).some((e) => e.id === id));
    if (i < 0) return false;
    this.catIndex = i;
    this.filters.set(this.cat().id, 0);
    this.query = '';
    this.select(id);
    return true;
  }

  /** 当前分类没有结果时，跳到第一个有结果的分类（筛选重置为全部） */
  jumpToFirstHit(): boolean {
    if (this.list().length > 0 || !this.query.trim()) return false;
    const i = this.cats.findIndex((_, k) => this.matchCount(k) > 0);
    if (i < 0) return false;
    this.catIndex = i;
    this.filters.set(this.cat().id, 0);
    return true;
  }
}
