// 标题菜单（法环标题画面式）：竖排按钮 + 柔光光标。鼠标真的移动才悬停选中，单击可用项即选中并执行（同 Enter）；
// ↑↓ 跳过不可用的项并循环，Enter 执行。可接一个「尾项」（横带里的动作），排在最后一项之后。
// 菜单不自己收键盘：由所在页面把按键转给 handleKey
import { onRealHover } from '../input/pointer';

export interface MenuItem { label: string; disabled?: boolean }

/** 尾项：菜单外的一个按钮，选中时光标隐去、按钮自己高亮 */
export interface MenuTail { el: HTMLElement; run: () => void }

export interface MenuOptions {
  items: MenuItem[];
  /** 初始选中（不可用时往后找） */
  initial?: number;
  onActivate: (index: number) => void;
  onSelect?: (index: number) => void;
}

export class Menu {
  readonly el: HTMLElement;
  private readonly opts: MenuOptions;
  private readonly items: MenuItem[];
  private readonly buttons: HTMLButtonElement[];
  private readonly cursor: HTMLElement;
  private readonly unhover: () => void;
  private tail: MenuTail | null = null;
  private sel = -1;

  constructor(opts: MenuOptions) {
    this.opts = opts;
    this.items = opts.items.map((item) => ({ ...item }));
    this.el = document.createElement('nav');
    this.el.className = 'tmenu';
    this.el.setAttribute('role', 'menu');
    this.cursor = document.createElement('div');
    this.cursor.className = 'tcur';
    const glow = document.createElement('i');
    glow.className = 'loop';
    this.cursor.append(glow);
    this.buttons = this.items.map((_, i) => {
      const b = document.createElement('button');
      b.type = 'button';
      b.tabIndex = -1;
      b.dataset.i = String(i);
      b.setAttribute('role', 'menuitem');
      b.style.animationDelay = `${0.55 + i * 0.07}s`;
      b.append(document.createElement('span'));
      return b;
    });
    this.el.append(this.cursor, ...this.buttons);
    this.el.addEventListener('click', (e) => this.onClick(e));
    this.unhover = onRealHover((t) => {
      const i = this.indexOf(t);
      if (i >= 0) this.apply(i);
    });
    this.paint();
    this.select(opts.initial ?? 0);
  }

  /** 当前选中的下标；选中尾项时等于项数，全部不可用时为 -1 */
  get selected(): number {
    return this.sel;
  }

  /** 选中 i；不可用时沿 dir 方向找下一个可用项（循环） */
  select(i: number, dir: 1 | -1 = 1): void {
    const n = this.items.length + (this.tail ? 1 : 0);
    for (let k = 0; k < n; k++) {
      const j = (((i + dir * k) % n) + n) % n;
      if (this.enabled(j)) return this.apply(j);
    }
    this.apply(-1);
  }

  move(dir: 1 | -1): void {
    this.select(this.sel + dir, dir);
  }

  activate(): void {
    const i = this.sel;
    if (!this.enabled(i)) return;
    if (i === this.items.length) this.tail?.run();
    else this.opts.onActivate(i);
  }

  /** ↑↓ 移动、Enter 执行；别的键不处理 */
  handleKey(key: string): boolean {
    if (key === 'ArrowUp') this.move(-1);
    else if (key === 'ArrowDown') this.move(1);
    else if (key === 'Enter') this.activate();
    else return false;
    return true;
  }

  /** 改某项的文字或可用状态（如「开始游戏」改成「切到游戏」） */
  setItem(i: number, patch: Partial<MenuItem>): void {
    if (!this.items[i]) return;
    Object.assign(this.items[i], patch);
    this.paint();
    if (!this.enabled(this.sel)) this.select(this.sel);
  }

  setTail(tail: MenuTail | null): void {
    if (this.tail === tail) return;
    if (this.tail) {
      this.tail.el.classList.remove('sel');
      this.tail.el.removeEventListener('click', this.onTailClick);
    }
    this.tail = tail;
    tail?.el.addEventListener('click', this.onTailClick);
    if (this.enabled(this.sel)) this.apply(this.sel);
    else this.select(Math.min(this.sel, this.items.length - 1));
  }

  destroy(): void {
    this.unhover();
    this.setTail(null);
    this.el.remove();
  }

  private readonly onTailClick = (e: MouseEvent) => this.clickItem(e, this.items.length);

  private onClick(e: MouseEvent): void {
    const i = this.indexOf(e.target);
    if (i >= 0 && i < this.items.length) this.clickItem(e, i);
  }

  /**
   * 单击：选中再执行，同 Enter。只认鼠标的第一下（detail 1）：双击的第二下（2）、
   * 按钮留着焦点时空格合成的点击（0）都不执行。切页后 500ms 内的点击已由 pointer 在捕获阶段丢弃
   */
  private clickItem(e: MouseEvent, i: number): void {
    if (e.detail !== 1) return;
    this.apply(i);
    this.activate();
  }

  private enabled(i: number): boolean {
    if (i === this.items.length) return !!this.tail;
    return i >= 0 && i < this.items.length && !this.items[i].disabled;
  }

  /** 目标落在哪个可用项上（含尾项）；没有为 -1 */
  private indexOf(t: EventTarget | null): number {
    if (!(t instanceof Element)) return -1;
    if (this.tail?.el.contains(t)) return this.items.length;
    const b = t.closest<HTMLElement>('button[data-i]');
    if (!b || !this.el.contains(b)) return -1;
    const i = Number(b.dataset.i);
    return this.enabled(i) ? i : -1;
  }

  private paint(): void {
    this.items.forEach((item, i) => {
      const b = this.buttons[i];
      b.firstElementChild!.textContent = item.label;
      b.classList.toggle('off', !!item.disabled);
      b.setAttribute('aria-disabled', String(!!item.disabled));
    });
  }

  private apply(i: number): void {
    const changed = i !== this.sel;
    this.sel = i;
    const onTail = i === this.items.length;
    this.buttons.forEach((b, k) => b.classList.toggle('sel', k === i));
    this.tail?.el.classList.toggle('sel', onTail);
    this.cursor.classList.toggle('off', i < 0 || onTail);
    if (i >= 0 && !onTail) this.cursor.style.setProperty('--i', String(i));
    if (changed && i >= 0) this.opts.onSelect?.(i);
  }
}
