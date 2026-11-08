// 图鉴的条目区：物品栏网格（每行 5 格）、缩略图列表、单行列表
import type { WikiEntry } from '../../api';
import { asset, el } from '../../ui/dom';
import { layoutOf, type Layout, type WikiModel } from './model';
import { hideOnError } from './panel';

export const GRID_COLS = 5;

/** 条目的图片地址，没有图时为 undefined */
export type EntryImage = (e: WikiEntry) => string | undefined;
/** 一行格子的高度（格 100 + 间距 3），用来算要补几行空格 */
const CELL_STEP = 103;
/** 选中项离上下边的留白 */
const SCROLL_TOP_GAP = 12;
const SCROLL_BOTTOM_GAP = 24;

export class WikiListing {
  readonly el = el('div', 'wbox');

  constructor() {
    this.el.addEventListener('scroll', () => this.fade());
  }

  /** 整个重画，并把选中项滚进可见范围 */
  paint(model: WikiModel, url: EntryImage): void {
    const list = model.list();
    const current = model.current();
    const layout = layoutOf(model.cat());
    this.el.scrollTop = 0;
    if (layout === 'grid') this.el.replaceChildren(this.grid(list, current, model, url));
    else this.el.replaceChildren(this.rows(list, current, layout, model, url));
    this.reveal();
  }

  /** 没有条目时显示两行字 */
  paintEmpty(a: string, b: string): void {
    const box = el('div', 'wnone');
    box.append(a, Object.assign(el('span'), { textContent: b }));
    this.el.replaceChildren(box);
    this.fade();
  }

  /** 只换选中样式；鼠标触发时不滚动，免得内容在静止的鼠标下移动 */
  mark(index: number, fromMouse: boolean): void {
    this.el.querySelector('.wcell.f, .row.sel')?.classList.remove('f', 'sel');
    const node = this.el.querySelector(`[data-i="${index}"]`);
    node?.classList.add(node.classList.contains('wcell') ? 'f' : 'sel');
    if (fromMouse) this.fade();
    else this.reveal();
  }

  /** 选中项保持在可见范围内 */
  reveal(): void {
    const node = this.el.querySelector<HTMLElement>('.wcell.f, .row.sel');
    if (node) {
      const top = node.offsetTop;
      const bottom = top + node.offsetHeight;
      const height = this.el.clientHeight;
      if (top < this.el.scrollTop + SCROLL_TOP_GAP) this.el.scrollTop = Math.max(0, top - SCROLL_TOP_GAP);
      else if (bottom > this.el.scrollTop + height - SCROLL_BOTTOM_GAP) this.el.scrollTop = bottom - height + SCROLL_BOTTOM_GAP;
    }
    this.fade();
  }

  /** 下面还有内容时底边渐隐 */
  private fade(): void {
    this.el.classList.toggle('dn', this.el.scrollTop + this.el.clientHeight < this.el.scrollHeight - 1);
  }

  private image(cls: string, src: string | undefined): HTMLImageElement {
    const img = el('img', cls);
    img.alt = '';
    if (src) img.src = src;
    hideOnError(img);
    return img;
  }

  private grid(list: WikiEntry[], current: WikiEntry | undefined, model: WikiModel, url: EntryImage): HTMLElement {
    const grid = el('div', 'wgrid');
    list.forEach((e, i) => {
      const b = el('button', e === current ? 'wcell f' : 'wcell');
      b.type = 'button';
      b.tabIndex = -1;
      b.dataset.i = String(i);
      b.setAttribute('aria-label', e.name);
      const cur = el('img', 'cur');
      cur.alt = '';
      cur.src = asset('sl_cursor.png');
      const pic = this.image(e.portrait ? 'i pt' : 'i', url(e));
      pic.loading = 'lazy';
      b.append(cur, pic);
      if (model.isNew(e.id)) b.append(this.newMark());
      grid.append(b);
    });
    const rows = Math.max(Math.ceil(list.length / GRID_COLS), Math.floor(this.el.clientHeight / CELL_STEP));
    for (let i = rows * GRID_COLS - list.length; i > 0; i--) grid.append(el('i', 'wcell e'));
    return grid;
  }

  private rows(list: WikiEntry[], current: WikiEntry | undefined, layout: Layout, model: WikiModel, url: EntryImage): HTMLElement {
    const ul = el('ul', 'wl');
    list.forEach((e, i) => {
      const li = el('li', `row${layout === 'thumbs' ? ' wth' : ''}${e === current ? ' sel' : ''}`);
      li.dataset.i = String(i);
      if (layout === 'thumbs') {
        const thumb = el('span', 'tb');
        thumb.append(this.image('col', url(e)));
        const text = el('span', 'tx');
        text.append(Object.assign(el('span', 'lb'), { textContent: e.name }), Object.assign(el('span', 'ln'), { textContent: e.desc ?? '' }));
        li.append(thumb, text);
      } else {
        const label = Object.assign(el('span', 'lb'), { textContent: e.name });
        if (e.sub) label.append(Object.assign(el('span', e.hand?.includes('sub') ? 'kd ph' : 'kd'), { textContent: e.sub }));
        li.append(this.image('ic', url(e)), label);
      }
      if (model.isNew(e.id)) li.append(this.newMark());
      ul.append(li);
    });
    return ul;
  }

  private newMark(): HTMLImageElement {
    const img = el('img', 'nw');
    img.alt = '新';
    img.src = asset('newmark.png');
    return img;
  }
}
