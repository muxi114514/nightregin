// 图鉴页（法环物品栏式）：分类、版式、筛选项都来自 wiki.json。
// 左边焦点分三区：0「显示」行、1「查找」行、2 条目区；右边说明面板
import '../../styles/wiki.css';
import type { WikiEntry, WikiOdds, WikiView } from '../../api';
import { pushKeyHandler } from '../../input/keys';
import { onRealHover } from '../../input/pointer';
import { el } from '../../ui/dom';
import { createHintBar, setHints } from '../../ui/hints';
import { TabBar } from '../../ui/tabs';
import { toastError } from '../../ui/toast';
import type { AppCtx, WikiFrom } from '../app';
import { renderControls, wikiHints } from './controls';
import { GRID_COLS, WikiListing } from './listing';
import { WikiModel, layoutOf } from './model';
import { WikiPanel, iconOf, type PanelCtx } from './panel';
import { WIKI_STATE_TEXT, WIKI_TEXT, notFoundText, searchMeta } from './texts';

export interface Wiki {
  el: HTMLElement;
  open(id?: string, from?: WikiFrom): void;
  close(): void;
  isOpen(): boolean;
}

type Zone = 0 | 1 | 2;

const HOME_ROW = 1;
const NO_ODDS: WikiOdds = { level: null, nightlords: null, maps: null, everdark: null };

export function createWiki(ctx: AppCtx): Wiki {
  const section = el('section', 'screen');
  section.id = 'codex';
  section.setAttribute('aria-label', '图鉴');
  const head = el('div', 'head');
  const name = el('h1');
  name.id = 'wname';
  head.append(name, el('i'));
  const rows = el('ul', 'olist');
  rows.id = 'wrows';
  const listing = new WikiListing();
  listing.el.id = 'wbox';
  const panel = new WikiPanel();
  const keys = createHintBar();
  keys.id = 'ckeys';
  section.append(head, rows, listing.el, panel.el, keys);

  let model: WikiModel | null = null;
  let view: WikiView | null = null;
  let tabs: TabBar | null = null;
  let release: (() => void) | null = null;
  let openSeq = 0;
  let loading: Promise<void> | null = null;
  let zone: Zone = 2;
  let editing = false;
  let oldQuery = '';
  /** 从哪里跳来的，Esc 回去；只记一层 */
  let from: WikiFrom | undefined;
  /** 焦点是被空分类顶到查找行的，下一个分类有条目时回条目区 */
  let raised = false;
  let odds: WikiOdds = NO_ODDS;
  /** 当前可见的条目（筛选 + 查找之后） */
  let visible: WikiEntry[] = [];

  const url = (rel: string) => ctx.api.wikiImageUrl(rel);
  const entryImage = (e: WikiEntry) => {
    const rel = iconOf(e, odds);
    return rel ? url(rel) : undefined;
  };
  const panelCtx = (m: WikiModel): PanelCtx => ({ odds, isNew: (id) => m.isNew(id), version: m.version, url });
  const input = () => rows.querySelector<HTMLInputElement>('#wq');
  const oneOption = (m: WikiModel) => m.options().length < 2;
  const currentIndex = (m: WikiModel) => Math.max(0, visible.indexOf(m.current() as WikiEntry));

  function buildTabs(m: WikiModel): void {
    const bar = new TabBar(
      m.cats.map((c) => ({ id: c.id, name: c.name, icon: '', iconUrl: c.icon ? url(c.icon) : undefined, badge: m.hasNew(c) })),
      (id) => {
        stopEdit();
        m.setCat(m.cats.findIndex((c) => c.id === id));
        if (raised && m.list().length > 0) {
          zone = 2;
          raised = false;
        }
        render();
      },
    );
    tabs = bar;
    section.prepend(bar.el);
  }

  /** 切分类、改筛选时整个重画 */
  function render(): void {
    const m = model;
    if (!m) return paintBad();
    tabs?.set(m.cat().id);
    name.textContent = m.cat().name;
    paintRows(m);
    paintBody(m);
  }

  /** 打字时只重画这部分，输入框不失焦 */
  function paintBody(m: WikiModel): void {
    visible = m.list();
    const q = m.query.trim();
    tabs?.dim(new Set(q ? m.cats.filter((_, i) => i !== m.catIndex && m.matchCount(i) === 0).map((c) => c.id) : []));
    if (zone === 2 && visible.length === 0) {
      zone = 1;
      raised = true;
    }
    if (visible.length === 0) {
      const [a, b] = emptyText(m);
      listing.paintEmpty(a, b);
    } else {
      listing.paint(m, entryImage);
    }
    paintPanel(m);
    markZone();
    paintHints();
  }

  function paintPanel(m: WikiModel): void {
    const e = m.current();
    if (e) panel.paint(m.cat(), e, panelCtx(m));
    else panel.paintNone(m.cat(), m.query.trim() ? searchMeta(m.query.trim()) : '', noneHint(m), url);
  }

  function noneHint(m: WikiModel): string {
    if (m.query.trim()) return editing ? WIKI_TEXT.noneEditing : WIKI_TEXT.noneSearch;
    return (m.cat().entries ?? []).length > 0 ? WIKI_TEXT.noneFilter : WIKI_TEXT.noneEmpty;
  }

  function emptyText(m: WikiModel): [string, string] {
    const q = m.query.trim();
    if (q) {
      const hits = m.cats.flatMap((c, i) => {
        const n = i === m.catIndex ? 0 : m.matchCount(i);
        return n > 0 ? [`${c.name} ${n} 条`] : [];
      });
      return notFoundText(q, hits, editing);
    }
    const c = m.cat();
    if ((c.entries ?? []).length > 0) return WIKI_TEXT.noMatch;
    const [a, b] = c.emptyText ?? WIKI_TEXT.noEntries;
    return [a ?? '', b ?? ''];
  }

  function paintRows(m: WikiModel): void {
    const single = oneOption(m);
    section.classList.toggle('one', single);
    if (single && zone === 0) zone = 1;
    rows.replaceChildren(...renderControls(m, editing));
  }

  function paintBad(): void {
    const state = view && view.state !== 'ok' ? view.state : 'broken';
    const [a, b] = WIKI_STATE_TEXT[state];
    name.textContent = '';
    rows.replaceChildren();
    listing.paintEmpty(a, b);
    panel.clear();
    setHints(keys, [{ icon: 'key_esc', label: WIKI_TEXT.back, key: 'Escape' }]);
  }

  function markZone(): void {
    for (const li of rows.children) li.classList.toggle('sel', Number((li as HTMLElement).dataset.z) === zone);
    listing.el.classList.toggle('on', zone === 2);
  }

  function paintHints(): void {
    const m = model;
    if (!m) return;
    const e = m.current();
    setHints(keys, wikiHints({
      zone,
      editing,
      detail: panel.isDetail(),
      grid: layoutOf(m.cat()) === 'grid',
      hasQuery: !!m.query,
      lord: e?.lord !== undefined,
      overflow: !!e && panel.overflows(),
    }));
  }

  function setZone(m: WikiModel, z: Zone, fromMouse = false): void {
    raised = false;
    zone = z === 2 && visible.length === 0 ? 1 : z === 0 && oneOption(m) ? 1 : z;
    markZone();
    paintHints();
    if (zone === 2 && !fromMouse) listing.reveal();
  }

  function pick(m: WikiModel, index: number, fromMouse = false): void {
    const entry = visible[index];
    if (!entry) return;
    m.select(entry.id);
    listing.mark(index, fromMouse);
    paintPanel(m);
    paintHints();
  }

  function setQuery(m: WikiModel, q: string): void {
    m.query = q;
    const field = input();
    if (field) field.value = q;
    paintBody(m);
  }

  function startEdit(m: WikiModel): void {
    const field = input();
    if (!field) return;
    editing = true;
    oldQuery = m.query;
    zone = 1;
    raised = false;
    field.readOnly = false;
    field.closest('.row')?.classList.add('edit');
    field.focus();
    field.setSelectionRange(field.value.length, field.value.length);
    markZone();
    paintHints();
  }

  /** cancel 时还原开始输入前的文字 */
  function stopEdit(cancel = false): void {
    const m = model;
    if (!editing || !m) return;
    editing = false;
    const field = input();
    if (field) {
      field.readOnly = true;
      field.closest('.row')?.classList.remove('edit');
      field.blur();
    }
    if (cancel && m.query !== oldQuery) setQuery(m, oldQuery);
    else if (m.list().length === 0) paintBody(m);
    markZone();
    paintHints();
  }

  /** 结束输入时当前分类没有结果，跳到第一个有结果的分类 */
  function jumpIfEmpty(m: WikiModel): void {
    if (m.jumpToFirstHit()) render();
  }

  function finishEdit(m: WikiModel): void {
    stopEdit();
    jumpIfEmpty(m);
    setZone(m, 2);
  }

  rows.addEventListener('input', (e) => {
    if (model && e.target === input()) {
      model.query = (e.target as HTMLInputElement).value;
      paintBody(model);
    }
  });
  // 焦点在输入框时 keys.ts 只分发 Esc，其余在这里处理；输入法组字中的按键交给输入法
  rows.addEventListener('keydown', (e) => {
    const m = model;
    if (!m || !editing || e.isComposing || e.keyCode === 229) return;
    if (e.key === 'Enter') finishEdit(m);
    else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      stopEdit();
      setZone(m, e.key === 'ArrowDown' ? 2 : 0);
    } else return;
    e.preventDefault();
  });
  // 左键点到别处即完成输入（右键不抢焦点）
  rows.addEventListener('focusout', (e) => {
    if (model && editing && e.target === input()) {
      stopEdit();
      jumpIfEmpty(model);
    }
  });

  function filterKey(m: WikiModel, k: string): boolean {
    if (k === 'ArrowLeft' || k === 'ArrowRight') {
      if (m.setFilter(k === 'ArrowLeft' ? -1 : 1)) render();
    } else if (k === 'ArrowDown') {
      setZone(m, 1);
    } else if (k !== 'ArrowUp') {
      return false;
    }
    return true;
  }

  function searchKey(m: WikiModel, k: string): boolean {
    if (k === 'ArrowUp') setZone(m, 0);
    else if (k === 'ArrowDown') setZone(m, 2);
    else if (k === 'Enter') startEdit(m);
    else if (k === 'Backspace' || k === 'Delete') {
      if (m.query) setQuery(m, '');
    } else if (!k.startsWith('Arrow')) return false;
    return true;
  }

  /** 带 lord 的条目进永夜王设置；本来就是从永夜王跳来的，等同返回并选中这一位 */
  function toLords(lord: number): void {
    if (from?.page === 'lords') {
      from = { page: 'lords', focus: lord };
      back();
      return;
    }
    ctx.go({ page: 'sys', tab: 'night', row: 'lords', lordFocus: lord });
  }

  function listKey(m: WikiModel, k: string): boolean {
    if (k === 'Enter') {
      const lord = m.current()?.lord;
      if (lord === undefined) return false;
      toLords(lord);
      return true;
    }
    if (!k.startsWith('Arrow')) return false;
    const grid = layoutOf(m.cat()) === 'grid';
    const cols = grid ? GRID_COLS : 1;
    const n = visible.length;
    const i = currentIndex(m);
    if (k === 'ArrowUp') {
      if (i - cols < 0) setZone(m, 1);
      else pick(m, i - cols);
    } else if (k === 'ArrowDown') {
      // 下一行不满时落到最后一格
      if (i + cols < n) pick(m, i + cols);
      else if (Math.floor(i / cols) < Math.floor((n - 1) / cols)) pick(m, n - 1);
    } else if (grid) {
      // 左右不换行
      const j = i + (k === 'ArrowLeft' ? -1 : 1);
      if (j >= 0 && j < n && Math.floor(j / cols) === Math.floor(i / cols)) pick(m, j);
    }
    return true;
  }

  function setDetail(on: boolean): void {
    panel.setDetail(on);
    paintHints();
  }

  /** 详细说明：↑↓ 滚动正文，R / Esc 收起 */
  function detailKey(k: string, low: string): boolean {
    if (k === 'ArrowDown' || k === 'ArrowUp') panel.scroll(k === 'ArrowDown' ? 1 : -1);
    else if (low === 'r' || k === 'Escape') setDetail(false);
    else return false;
    return true;
  }

  function back(): void {
    stopEdit();
    const origin = from;
    from = undefined;
    if (origin?.page === 'lords') ctx.go({ page: 'sys', tab: 'night', row: 'lords', lordFocus: origin.focus });
    else if (origin) ctx.go({ page: 'sys', tab: origin.tab, row: origin.row });
    else ctx.go({ page: 'home', row: HOME_ROW });
  }

  /** 鼠标点过的按钮留着焦点，Enter / 空格交给浏览器会把它再点一次 */
  function onKey(k: string): boolean {
    return route(k) || k === 'Enter' || k === ' ';
  }

  function route(k: string): boolean {
    const m = model;
    if (editing && m) {
      // 输入框里只有 Esc 会到这里；按键提示的点击走同一条路
      if (k === 'Escape') stopEdit(true);
      else if (k === 'Enter') finishEdit(m);
      return true;
    }
    const low = k.toLowerCase();
    if (panel.isDetail()) return detailKey(k, low);
    if (k === 'Escape') {
      back();
      return true;
    }
    if (!m) return false;
    if (low === 'r' && panel.overflows() && m.current()) {
      setDetail(true);
      return true;
    }
    if (low === 'q' || low === 'e') {
      tabs?.move(low === 'q' ? -1 : 1);
      return true;
    }
    if (zone === 0) return filterKey(m, k);
    if (zone === 1) return searchKey(m, k);
    return listKey(m, k);
  }

  // 鼠标：移动指针即选中（打字时不抢焦点），单击只选中；「显示」行点箭头改值，「查找」行单击开始输入
  onRealHover((t) => {
    const m = model;
    if (!m || !release || editing) return;
    const cell = t.closest<HTMLElement>('#wbox .wcell[data-i], #wbox .row[data-i]');
    const row = t.closest<HTMLElement>('#wrows .row');
    if (cell) {
      const j = Number(cell.dataset.i);
      if (zone !== 2) setZone(m, 2, true);
      if (visible[j] && visible[j] !== m.current()) pick(m, j, true);
    } else if (row && Number(row.dataset.z) !== zone) {
      setZone(m, Number(row.dataset.z) as Zone);
    }
  });
  listing.el.addEventListener('click', (e) => {
    const m = model;
    const cell = e.target instanceof Element ? e.target.closest<HTMLElement>('.wcell[data-i], .row[data-i]') : null;
    if (!m || !cell || e.detail > 1) return;
    setZone(m, 2, true);
    pick(m, Number(cell.dataset.i), true);
  });
  rows.addEventListener('click', (e) => {
    const m = model;
    const row = e.target instanceof Element ? e.target.closest<HTMLElement>('.row') : null;
    if (!m || !row) return;
    if (row.dataset.z === '0') {
      setZone(m, 0);
      const arrow = (e.target as Element).closest('.ar');
      if (arrow && m.setFilter(arrow.classList.contains('l') ? -1 : 1)) render();
    } else if (!editing) {
      startEdit(m);
    }
  });

  /** 第一次读数据；还在读时再打开页面共用同一次，不会建出两个标签栏 */
  function loadData(): Promise<void> {
    if (model) return Promise.resolve();
    loading ??= ctx.api.getWiki()
      .then((v) => {
        view = v;
        if (v.state !== 'ok') return;
        const m = new WikiModel(v.data, v.newIds);
        if (m.cats.length === 0) {
          view = { state: 'broken', message: '没有可用的分类' };
          return;
        }
        model = m;
        buildTabs(m);
      })
      .finally(() => {
        loading = null;
      });
    return loading;
  }

  async function load(): Promise<void> {
    await loadData();
    if (model) odds = await ctx.api.getWikiOdds();
  }

  return {
    el: section,
    isOpen: () => release !== null,
    open(id, origin) {
      if (!release) release = pushKeyHandler(onKey);
      from = origin;
      section.classList.add('on');
      const seq = ++openSeq;
      load()
        .catch((err) => {
          toastError(err);
          view = { state: 'broken', message: String(err) };
        })
        .then(() => {
          if (seq !== openSeq) return;
          if (model) {
            if (id) {
              if (model.focusEntry(id)) zone = 2;
            } else if (!origin) {
              model.query = '';
            }
          }
          render();
        })
        .catch(toastError);
    },
    close() {
      openSeq++;
      stopEdit();
      release?.();
      release = null;
      section.classList.remove('on');
    },
  };
}
