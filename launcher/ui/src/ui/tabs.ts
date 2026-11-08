// 次级页顶部的标签横幅：石雕图标 + 文字，Q / E 切换，到头回绕；选中标签由 set 决定；放不下时跟着选中项滚动
import { pressKey } from '../input/keys';
import { asset, el } from './dom';

/** icon 是 public\assets 下的文件名；数据驱动的页面用 iconUrl，badge 显示「新」角标 */
export interface TabDef { id: string; name: string; icon: string; iconUrl?: string; badge?: boolean }

/** 每个标签至少要这么宽，低于它就改成固定宽度、跟着选中项滚动（同 sys.css 的 .tabs.many） */
const SCROLL_TAB_WIDTH = 64;

export class TabBar {
  readonly el: HTMLElement;
  private readonly tabs: TabDef[];
  private readonly buttons: HTMLButtonElement[];
  private readonly box: HTMLElement;
  private readonly row: HTMLElement;
  private readonly onChange: (id: string) => void;
  private current = 0;

  constructor(tabs: TabDef[], onChange: (id: string) => void) {
    this.tabs = tabs;
    this.onChange = onChange;
    this.el = el('div', 'tabwrap');
    this.box = el('div', 'tabs');
    const base = el('img');
    base.alt = '';
    base.src = asset('tab_base.png');
    const view = el('div', 'tabview');
    this.row = el('div', 'tabrow');
    this.buttons = tabs.map((t, i) => this.button(t, i));
    this.row.append(...this.buttons);
    view.append(this.row);
    this.box.append(base, view);
    this.el.append(this.box, this.nav('l', 'key_q', 'q'), this.nav('r', 'key_e', 'e'));
  }

  private button(t: TabDef, i: number): HTMLButtonElement {
    const b = el('button', 'tab');
    b.type = 'button';
    b.tabIndex = -1;
    const cursor = el('img', 'tc');
    cursor.alt = '';
    cursor.src = asset('tab_cursor.png');
    const icon = el('img', 'ic');
    icon.alt = '';
    icon.src = t.iconUrl ?? asset(`${t.icon}.png`);
    const label = Object.assign(el('span', 'tl'), { textContent: t.name });
    if (t.badge) {
      const mark = el('img', 'nb');
      mark.alt = '有新增';
      mark.src = asset('newmark.png');
      label.append(mark);
    }
    b.append(cursor, icon, label);
    b.addEventListener('click', (e) => {
      if (e.detail <= 1) this.pick(i);
    });
    return b;
  }

  private nav(side: 'l' | 'r', icon: string, key: string): HTMLButtonElement {
    const b = el('button', `tnav ${side}`);
    b.type = 'button';
    b.tabIndex = -1;
    const k = el('img', 'k');
    k.alt = key.toUpperCase();
    k.src = asset(`${icon}.png`);
    const a = el('img', 'a');
    a.alt = '';
    a.src = asset('arrow.png');
    if (side === 'l') b.append(k, a);
    else b.append(a, k);
    b.addEventListener('click', (e) => {
      if (e.detail <= 1) pressKey(key);
    });
    return b;
  }

  set(id: string): void {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    this.current = i;
    this.buttons.forEach((b, k) => b.classList.toggle('on', k === i));
    this.follow();
  }

  /** 这些标签变暗（查找时没有结果的分类） */
  dim(ids: Set<string>): void {
    this.buttons.forEach((b, i) => b.classList.toggle('none', ids.has(this.tabs[i].id)));
  }

  /** 放得下就不动；放不下时固定标签宽度，选中项尽量居中 */
  private follow(): void {
    const view = this.row.clientWidth;
    const total = this.buttons.length * SCROLL_TAB_WIDTH;
    const many = total > view;
    this.box.classList.toggle('many', many);
    const x = many ? Math.max(0, Math.min(total - view, (this.current + 0.5) * SCROLL_TAB_WIDTH - view / 2)) : 0;
    this.row.style.transform = x ? `translateX(${-x}px)` : '';
  }

  move(dir: 1 | -1): void {
    const n = this.tabs.length;
    this.pick((this.current + dir + n) % n);
  }

  private pick(i: number): void {
    if (i !== this.current) this.onChange(this.tabs[i].id);
  }
}
