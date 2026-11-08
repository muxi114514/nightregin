// 法环提示横带（草图 v4 的 setIssue）：横贯全宽的 Dialog 横带，一句文案 + 小字 + 可选的一个动作按钮。
// 动作按钮的选中与执行交给菜单：show() 返回的尾项传给 Menu.setTail

export interface BannerContent {
  msg: string;
  sub?: string;
  action?: { label: string; run: () => void };
}

export class Banner {
  readonly el: HTMLElement;
  private readonly msgEl: HTMLElement;
  private readonly subEl: HTMLElement;
  private readonly actionEl: HTMLButtonElement;
  private readonly actionLabel: HTMLElement;

  constructor() {
    this.el = document.createElement('div');
    this.el.className = 'band issue';
    this.el.hidden = true;
    const line = document.createElement('p');
    this.msgEl = document.createElement('b');
    this.subEl = document.createElement('span');
    line.append(Object.assign(document.createElement('i'), { className: 'bang' }), this.msgEl, this.subEl);
    this.actionEl = document.createElement('button');
    this.actionEl.type = 'button';
    this.actionEl.className = 'opt';
    this.actionEl.tabIndex = -1;
    this.actionLabel = document.createElement('span');
    this.actionEl.append(this.actionLabel);
    this.el.append(line, this.actionEl);
  }

  get visible(): boolean {
    return !this.el.hidden;
  }

  /** 显示一条（null 隐藏）；有动作时返回给菜单的尾项，否则 null */
  show(c: BannerContent | null): { el: HTMLElement; run: () => void } | null {
    this.el.hidden = !c;
    if (!c) return null;
    this.msgEl.textContent = c.msg;
    this.subEl.textContent = c.sub ?? '';
    this.subEl.hidden = !c.sub;
    this.actionEl.hidden = !c.action;
    if (!c.action) return null;
    this.actionLabel.textContent = c.action.label;
    return { el: this.actionEl, run: c.action.run };
  }
}
