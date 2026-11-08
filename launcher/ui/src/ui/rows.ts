// 法环式单行选项列表：取值行「‹ 值 ›」，单击只选中，改值用 ← → 或行内箭头，到边界停住；
// 动作行单击执行（双击的第二下不算）；分隔行不可选；刷新时按行的 id 找回光标
import { onRealHover } from '../input/pointer';
import type { Desc } from './desc';
import { asset, el } from './dom';

export type Value<T> = { ok: true; value: T } | { ok: false; message: string };

interface Base {
  id: string;
  label: string;
  off?(): boolean;
  desc(): Desc;
}

export type Row =
  | (Base & { type: 'toggle'; get(): Value<boolean>; set(v: boolean): void })
  | (Base & { type: 'number'; min: number; max: number; step: number; unit: string; get(): Value<number>; set(v: number): void })
  | (Base & { type: 'pick'; options: string[]; get(): Value<number>; set(i: number): void })
  | (Base & {
      type: 'action';
      value?(): string;
      enter?(): string;
      run(): void;
      /** 双击也算两下（「关于」连按 5 次） */
      rawClicks?: boolean;
      /** 读取失败的原因；有值时显示「读取失败」 */
      failed?(): string | null;
      /** 有它时显示「‹ 值 ›」，← → 走这里 */
      step?(dir: 1 | -1): void;
      atEnd?(): { lo: boolean; hi: boolean };
    })
  | (Base & { type: 'path'; value(): string; enter: string; run(): void })
  | (Base & { type: 'readonly'; value(): string })
  | { type: 'separator'; id: string };

export type Selectable = Exclude<Row, { type: 'separator' }>;

export const FAILED_TEXT = '读取失败';

/** 按步长走一格：手改到网格外的值先回到网格（125 按 ← 到 120），超范围的回到边界 */
export function stepValue(cur: number, dir: 1 | -1, min: number, max: number, step: number): number {
  const next = dir < 0
    ? Math.floor((cur - 1 - min) / step) * step + min
    : Math.ceil((cur + 1 - min) / step) * step + min;
  return Math.min(max, Math.max(min, next));
}

const selectable = (r: Row): r is Selectable => r.type !== 'separator';

/** 这一行读取失败的原因；没失败为 null */
export function rowFailure(r: Selectable): string | null {
  if (r.type === 'action') return r.failed?.() ?? null;
  if (r.type === 'toggle' || r.type === 'number' || r.type === 'pick') {
    const v = r.get();
    return v.ok ? null : v.message;
  }
  return null;
}

export class RowList {
  readonly el = el('ul', 'olist');
  /** 滚动条，放在列表旁边 */
  readonly bar = el('div', 'sbar');
  private readonly thumb = el('i');
  private rows: Row[] = [];
  private sel = -1;
  private readonly unhover: () => void;

  private readonly o: { onSelect(r: Selectable): void; onFailed(r: Selectable): void };

  constructor(o: { onSelect(r: Selectable): void; onFailed(r: Selectable): void }) {
    this.o = o;
    this.bar.hidden = true;
    this.bar.append(this.thumb);
    this.el.addEventListener('scroll', () => this.paintBar());
    this.el.addEventListener('click', (e) => this.onClick(e));
    this.unhover = onRealHover((t) => {
      const i = this.indexOf(t);
      if (i >= 0 && i !== this.sel) this.select(i, 1, true);
    });
  }

  /** 换整组行；keepId 所在的行还在就留在那里，滚动位置不变 */
  setRows(rows: Row[], keepId?: string): void {
    const top = this.el.scrollTop;
    this.rows = rows;
    this.el.replaceChildren(...rows.map((r) => this.renderRow(r)));
    const kept = keepId === undefined ? -1 : rows.findIndex((r) => r.id === keepId && selectable(r));
    const first = rows.findIndex(selectable);
    this.el.scrollTop = top;
    if (kept >= 0) this.select(kept, 1, false);
    else if (first >= 0) this.select(first, 1, false);
    else this.sel = -1;
    this.paintBar();
  }

  /** 只重画值：光标和滚动位置不动 */
  refresh(): void {
    const top = this.el.scrollTop;
    this.rows.forEach((r, i) => {
      const li = this.renderRow(r);
      li.classList.toggle('sel', i === this.sel);
      this.el.children[i]?.replaceWith(li);
    });
    this.el.scrollTop = top;
    const r = this.rows[this.sel];
    if (r && selectable(r)) this.o.onSelect(r);
  }

  selectedId(): string | null {
    return this.rows[this.sel]?.id ?? null;
  }

  handleKey(key: string): boolean {
    switch (key) {
      case 'ArrowUp': this.move(-1); return true;
      case 'ArrowDown': this.move(1); return true;
      case 'ArrowLeft': this.change(-1); return true;
      case 'ArrowRight': this.change(1); return true;
      case 'Enter': this.enter(); return true;
      default: return false;
    }
  }

  destroy(): void {
    this.unhover();
  }

  private move(dir: 1 | -1): void {
    if (this.sel >= 0) this.select(this.sel + dir, dir, false);
  }

  /** mouse：鼠标选中时不滚动，免得列表在静止的鼠标下移动 */
  private select(i: number, dir: 1 | -1, mouse: boolean): void {
    const n = this.rows.length;
    if (!n) return;
    let k = ((i % n) + n) % n;
    for (let guard = 0; guard < n && !selectable(this.rows[k]); guard++) k = (k + dir + n) % n;
    const r = this.rows[k];
    if (!selectable(r)) return;
    this.sel = k;
    [...this.el.children].forEach((li, j) => li.classList.toggle('sel', j === k));
    if (!mouse) this.scrollTo(this.el.children[k] as HTMLElement);
    this.paintBar();
    this.o.onSelect(r);
  }

  private scrollTo(li: HTMLElement): void {
    const margin = 36;
    const top = li.offsetTop;
    const bottom = top + li.offsetHeight;
    const h = this.el.clientHeight;
    if (top < this.el.scrollTop + margin) this.el.scrollTop = Math.max(0, top - margin);
    else if (bottom > this.el.scrollTop + h - margin) this.el.scrollTop = bottom - h + margin;
  }

  private paintBar(): void {
    const h = this.el.clientHeight;
    const total = this.el.scrollHeight;
    const more = total > h + 1;
    this.bar.hidden = !more;
    this.el.classList.toggle('up', more && this.el.scrollTop > 1);
    this.el.classList.toggle('dn', more && this.el.scrollTop + h < total - 1);
    if (!more) return;
    const thumb = Math.max(48, (h * h) / total);
    this.thumb.style.height = `${thumb}px`;
    this.thumb.style.transform = `translateY(${(this.el.scrollTop * (h - thumb)) / (total - h)}px)`;
  }

  private current(): Selectable | null {
    const r = this.rows[this.sel];
    return r && selectable(r) ? r : null;
  }

  private change(dir: 1 | -1): void {
    const r = this.current();
    if (!r || r.off?.() || rowFailure(r) !== null) return;
    if (r.type === 'toggle') {
      const v = r.get();
      if (v.ok && v.value !== (dir > 0)) r.set(dir > 0);
    } else if (r.type === 'number') {
      const v = r.get();
      if (!v.ok) return;
      const next = stepValue(v.value, dir, r.min, r.max, r.step);
      if (next !== v.value) r.set(next);
    } else if (r.type === 'pick') {
      const v = r.get();
      if (!v.ok) return;
      const next = Math.max(0, Math.min(r.options.length - 1, v.value + dir));
      if (next !== v.value) r.set(next);
    } else if (r.type === 'action' && r.step) {
      r.step(dir);
    } else {
      return;
    }
    this.repaint();
  }

  private enter(): void {
    const r = this.current();
    if (!r || r.off?.()) return;
    if (rowFailure(r) !== null) {
      this.o.onFailed(r);
    } else if (r.type === 'toggle') {
      const v = r.get();
      if (v.ok) {
        r.set(!v.value);
        this.repaint();
      }
    } else if (r.type === 'action' || r.type === 'path') {
      r.run();
    }
  }

  private repaint(): void {
    // 改值可能影响别的行（强制深夜关掉时层数变暗），整组重画
    this.refresh();
  }

  private indexOf(t: EventTarget | null): number {
    if (!(t instanceof Element)) return -1;
    const li = t.closest<HTMLElement>('.row');
    if (!li || !this.el.contains(li) || li.classList.contains('hd')) return -1;
    return [...this.el.children].indexOf(li);
  }

  private onClick(e: MouseEvent): void {
    const i = this.indexOf(e.target);
    if (i < 0) return;
    if (i !== this.sel) this.select(i, 1, true);
    const r = this.current();
    if (!r) return;
    const arrow = e.target instanceof Element ? e.target.closest<HTMLElement>('.ar') : null;
    if (arrow) {
      this.change(arrow.classList.contains('l') ? -1 : 1);
    } else if ((r.type === 'action' && (r.rawClicks || e.detail === 1)) || (r.type === 'path' && e.detail === 1)) {
      this.enter();
    }
  }

  private renderRow(r: Row): HTMLLIElement {
    const li = el('li', `row ${r.type}`);
    li.dataset.id = r.id;
    if (!selectable(r)) {
      li.classList.add('hd');
      li.append(el('i'));
      return li;
    }
    if (r.off?.()) li.classList.add('off');
    li.append(Object.assign(el('span', 'lb'), { textContent: r.label }), this.valueOf(r));
    return li;
  }

  private valueOf(r: Selectable): HTMLElement {
    if (rowFailure(r) !== null) return small(FAILED_TEXT);
    const off = !!r.off?.();
    switch (r.type) {
      case 'toggle': {
        const v = r.get();
        return v.ok ? arrows(v.value ? '开启' : '关闭', !v.value, v.value) : small(FAILED_TEXT);
      }
      case 'number': {
        const v = r.get();
        return v.ok ? arrows(`${v.value}${r.unit}`, off || v.value <= r.min, off || v.value >= r.max) : small(FAILED_TEXT);
      }
      case 'pick': {
        const v = r.get();
        return v.ok ? arrows(r.options[v.value] ?? '', v.value <= 0, v.value >= r.options.length - 1) : small(FAILED_TEXT);
      }
      case 'action': {
        const text = r.value?.() ?? '';
        if (r.step) {
          const ends = r.atEnd?.() ?? { lo: false, hi: false };
          return arrows(text, ends.lo, ends.hi);
        }
        return text ? small(text) : el('span');
      }
      case 'path':
        return Object.assign(el('span', 'pth'), { textContent: r.value() });
      case 'readonly': {
        const box = el('span', 'val');
        box.append(Object.assign(el('span', 'v'), { textContent: r.value() }));
        return box;
      }
    }
  }
}

function small(text: string): HTMLElement {
  const box = el('span', 'val');
  box.append(Object.assign(el('span', 'v s'), { textContent: text }));
  return box;
}

function arrow(side: 'l' | 'r', end: boolean): HTMLElement {
  const a = el('span', `ar ${side}${end ? ' end' : ''}`);
  const img = el('img');
  img.alt = side === 'l' ? '减少' : '增加';
  img.src = asset('arrow.png');
  a.append(img);
  return a;
}

export function arrows(text: string, lo: boolean, hi: boolean): HTMLElement {
  const box = el('span', 'val');
  box.append(arrow('l', lo), Object.assign(el('span', 'v'), { textContent: text }), arrow('r', hi));
  return box;
}
