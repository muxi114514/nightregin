// 次级页容器（法环系统菜单）：标签横幅、页名、左边单行选项、右边说明面板、底部按键提示。
// 页面只提供行和数据；永夜王这样的子层放进 grid，按键交给页面
import '../styles/sys.css';
import { inputGuard } from '../input/guard';
import { pushKeyHandler } from '../input/keys';
import { renderDesc, type Desc } from '../ui/desc';
import { el } from '../ui/dom';
import { createHintBar, setHints, type Hint } from '../ui/hints';
import { RowList, rowFailure, type Row, type Selectable } from '../ui/rows';
import { TabBar } from '../ui/tabs';
import { toastError } from '../ui/toast';
import type { AppCtx } from './app';

export type SysTab = 'night' | 'options' | 'saves';

export const MENU_ROW: Record<SysTab, number> = { night: 2, options: 3, saves: 4 };

const TABS: { id: SysTab; name: string; icon: string }[] = [
  { id: 'night', name: '深夜与联机', icon: 'tab_net' },
  { id: 'options', name: '设置', icon: 'tab_option' },
  { id: 'saves', name: '存档', icon: 'tab_save' },
];

/** 页面能对容器做的事 */
export interface SysHost {
  /** 只重画值，光标不动 */
  refresh(): void;
  /** 行的结构变了：重新取行，按 id 找回光标 */
  reload(keepId?: string): void;
  /** 子层（永夜王网格）放这里 */
  grid: HTMLElement;
  setTitle(text: string, count?: string): void;
  setDesc(title: string, d: Desc): void;
  setHints(hints: Hint[]): void;
  enterSub(): void;
  exitSub(rowId?: string): void;
}

export interface SysPage {
  rows(): Row[];
  defaultRow: string;
  load(): Promise<void>;
  attach?(host: SysHost): void;
  hints?(r: Selectable): Hint[] | null;
  /** 子层打开时收全部按键；平时可以先截一部分 */
  handleKey?(key: string): boolean;
  /** Enter 在读取失败的行上 */
  onFailed?(r: Selectable): void;
}

export interface Sys {
  el: HTMLElement;
  open(tab: SysTab, row?: string): void;
  close(): void;
  isOpen(): boolean;
}

const isValueRow = (r: Selectable) =>
  r.type === 'toggle' || r.type === 'number' || r.type === 'pick' || (r.type === 'action' && !!r.step);

function enterLabel(r: Selectable): string {
  if (r.type === 'toggle') return '切换';
  if (r.type === 'path') return r.enter;
  if (r.type === 'action') return r.enter ? r.enter() : '确定';
  return '';
}

export function createSys(ctx: AppCtx, pages: Record<SysTab, SysPage>): Sys {
  const section = el('section', 'screen');
  section.id = 'sys';
  section.setAttribute('aria-label', '系统');
  const head = el('div', 'head');
  const title = el('h1');
  head.append(title, el('i'));
  const grid = el('div', 'lgrid');
  grid.hidden = true;
  const desc = el('aside', 'desc');
  desc.setAttribute('aria-live', 'polite');
  const keys = createHintBar();
  keys.id = 'skeys';

  let tab: SysTab = 'night';
  let inSub = false;
  let release: (() => void) | null = null;
  let loadSeq = 0;
  const lastRow: Partial<Record<SysTab, string>> = {};
  const page = () => pages[tab];
  const tabName = () => TABS.find((t) => t.id === tab)?.name ?? '';

  const tabs = new TabBar(TABS, (id) => void show(id as SysTab));
  const list = new RowList({
    onSelect: (r) => {
      if (inSub) return;
      lastRow[tab] = r.id;
      renderDesc(desc, r.label, r.desc(), ctx.devMode());
      paintHints(r);
    },
    onFailed: (r) => page().onFailed?.(r),
  });
  section.append(tabs.el, head, list.el, grid, list.bar, desc, keys);

  function paintHints(r: Selectable): void {
    const custom = page().hints?.(r);
    if (custom) return setHints(keys, custom);
    const hints: Hint[] = [{ icon: 'key_ud', label: '选择' }];
    const failed = rowFailure(r) !== null;
    if (!failed && !r.off?.() && isValueRow(r)) hints.push({ icon: 'key_lr', label: '调整', key: 'ArrowRight', alt: 'ArrowLeft' });
    const enter = failed ? '恢复默认' : enterLabel(r);
    if (enter && !r.off?.()) hints.push({ icon: 'key_enter', label: enter, key: 'Enter' });
    hints.push({ icon: 'key_esc', label: '返回', key: 'Escape' });
    setHints(keys, hints);
  }

  const host: SysHost = {
    refresh: () => list.refresh(),
    reload: (keepId) => list.setRows(page().rows(), keepId ?? list.selectedId() ?? undefined),
    grid,
    setTitle: (text, count) => {
      title.textContent = text;
      if (count) title.append(Object.assign(el('span', 'cnt'), { textContent: count }));
    },
    setDesc: (t, d) => renderDesc(desc, t, d, ctx.devMode()),
    setHints: (h) => setHints(keys, h),
    enterSub: () => {
      inSub = true;
      section.classList.add('lords');
      list.el.hidden = true;
      list.bar.hidden = true;
      grid.hidden = false;
      inputGuard.markLayerChange(performance.now());
      inputGuard.markSublayerEnter(performance.now());
    },
    exitSub: (rowId) => {
      inSub = false;
      section.classList.remove('lords');
      grid.hidden = true;
      list.el.hidden = false;
      inputGuard.markLayerChange(performance.now());
      inputGuard.markSublayerEnter(performance.now());
      host.setTitle(tabName());
      list.setRows(page().rows(), rowId ?? lastRow[tab]);
    },
  };
  for (const p of Object.values(pages)) p.attach?.(host);

  function show(next: SysTab, row?: string): void {
    tab = next;
    const seq = ++loadSeq;
    tabs.set(tab);
    host.setTitle(tabName());
    inputGuard.markLayerChange(performance.now());
    const paint = () => {
      if (seq !== loadSeq) return;
      list.setRows(page().rows(), row ?? lastRow[tab] ?? page().defaultRow);
    };
    list.el.scrollTop = 0;
    paint();
    page().load().then(paint).catch(toastError);
  }

  function onKey(k: string): boolean {
    if (inSub) {
      page().handleKey?.(k);
      return true;
    }
    if (k === 'Escape') {
      ctx.go({ page: 'home', row: MENU_ROW[tab] });
      return true;
    }
    const low = k.toLowerCase();
    if (low === 'q' || low === 'e') {
      tabs.move(low === 'q' ? -1 : 1);
      return true;
    }
    if (page().handleKey?.(k)) return true;
    return list.handleKey(k);
  }

  return {
    el: section,
    open(next, row) {
      if (!release) release = pushKeyHandler(onKey);
      section.classList.add('on');
      show(next, row);
    },
    isOpen: () => release !== null,
    close() {
      release?.();
      release = null;
      section.classList.remove('on');
      if (inSub) host.exitSub();
    },
  };
}
