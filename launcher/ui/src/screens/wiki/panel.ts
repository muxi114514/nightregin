// 图鉴的说明面板：按字段通用显示；正文溢出时出现细滚动条，R 展开「详细说明」
import type { WikiCategory, WikiEntry, WikiOdds } from '../../api';
import { asset, el } from '../../ui/dom';
import { NOTE, appearText, everText, showsEver } from './odds';
import { WIKI_TEXT, newTagText } from './texts';

export interface PanelCtx {
  odds: WikiOdds;
  isNew(id: string): boolean;
  /** wiki.json 的 version，「新」标记旁边显示 */
  version?: string;
  url(rel: string): string;
}

/** 短的两两并排，长的占一整行 */
const STAT_WIDE_VALUE = 9;
const STAT_WIDE_KEY = 5;
const SCROLL_STEP = 72;
const MIN_THUMB = 28;

/** 永夜状态决定用哪张头像 */
export const iconOf = (e: WikiEntry, odds: WikiOdds) => (e.everIcon && showsEver(e, odds) ? e.everIcon : e.icon);

/** 图片缺失时只留空位，不露破图标 */
export function hideOnError(img: HTMLImageElement): void {
  img.addEventListener('error', () => img.classList.add('gone'), { once: true });
}

const para = (cls: string, text: string) => Object.assign(el('p', cls), { textContent: text });

export class WikiPanel {
  readonly el = el('aside', 'desc wk');
  private body: HTMLElement | null = null;
  private bar: HTMLElement | null = null;
  private overflow = false;

  constructor() {
    this.el.id = 'wdesc';
    this.el.setAttribute('aria-live', 'polite');
  }

  paint(c: WikiCategory, e: WikiEntry, ctx: PanelCtx): void {
    this.setClass(false);
    const hand = new Set(e.hand ?? []);
    const handStat = (e.stats ?? []).some((s) => s.h);
    const body = this.bodyOf(e, ctx, hand);
    const appear = appearText(e, ctx.odds);
    const foot = el('div', 'wfoot');
    const source = appear ?? e.source;
    if (source) foot.append(this.sourceLine(c, source, !appear && hand.has('source')));
    if (appear) foot.append(para('wnote', NOTE));
    if (hand.size > 0 || handStat) foot.append(para('dt ph', WIKI_TEXT.handNote));
    const bar = el('div', 'wsb');
    bar.hidden = true;
    bar.append(el('i'));
    this.el.replaceChildren(this.headOf(e, ctx), el('i', 'sep'), body, bar, foot);
    this.body = body;
    this.bar = bar;
    body.addEventListener('scroll', () => this.syncBar());
    this.syncBar();
    this.overflow = body.scrollHeight > body.clientHeight + 1;
  }

  /** 没有条目时：分类图标、名称、一句提示 */
  paintNone(c: WikiCategory, meta: string, hint: string, url: (rel: string) => string): void {
    this.setClass(false);
    this.body = null;
    this.bar = null;
    this.overflow = false;
    const head = el('div', 'dh');
    const pic = el('div', 'pic box');
    if (c.icon) pic.append(this.image(url(c.icon)));
    const title = el('div');
    title.append(Object.assign(el('h2'), { textContent: c.name }));
    if (meta) title.append(para('meta', meta));
    head.append(pic, title);
    const body = el('div', 'wbody');
    body.append(para('', hint));
    this.el.replaceChildren(head, el('i', 'sep'), body);
  }

  clear(): void {
    this.setClass(false);
    this.body = null;
    this.bar = null;
    this.overflow = false;
    this.el.replaceChildren();
  }

  overflows(): boolean {
    return this.overflow;
  }

  isDetail(): boolean {
    return this.el.classList.contains('det');
  }

  setDetail(on: boolean): void {
    this.setClass(on);
    this.syncBar();
  }

  scroll(dir: 1 | -1): void {
    this.body?.scrollBy({ top: dir * SCROLL_STEP });
  }

  private setClass(detail: boolean): void {
    this.el.classList.toggle('det', detail);
  }

  private image(src: string): HTMLImageElement {
    const img = el('img');
    img.alt = '';
    img.src = src;
    hideOnError(img);
    return img;
  }

  private headOf(e: WikiEntry, ctx: PanelCtx): HTMLElement {
    const head = el('div', 'dh');
    const pic = el('div', e.portrait ? 'pic rd' : 'pic box');
    const icon = iconOf(e, ctx.odds);
    if (icon) pic.append(this.image(ctx.url(icon)));
    const text = el('div');
    text.append(Object.assign(el('h2'), { textContent: e.name }));
    if (e.sub) text.append(para(e.hand?.includes('sub') ? 'meta ph' : 'meta', e.sub));
    if (ctx.isNew(e.id)) {
      const tag = el('p', 'wtag');
      const mark = el('img', 'nw');
      mark.alt = '新';
      mark.src = asset('newmark.png');
      tag.append(mark, newTagText(ctx.version));
      text.append(tag);
    }
    head.append(pic, text);
    return head;
  }

  private bodyOf(e: WikiEntry, ctx: PanelCtx, hand: Set<string>): HTMLElement {
    const body = el('div', 'wbody');
    const ever = everText(e, ctx.odds);
    if (ever) {
      const line = el('p', 'ever');
      const icon = iconOf(e, ctx.odds);
      if (icon) line.append(this.image(ctx.url(icon)));
      line.append(ever);
      body.append(line);
    }
    if (e.stats?.length) body.append(this.stats(e));
    const desc = e.desc ?? '';
    if (desc) body.append(para(hand.has('desc') ? 'ph' : '', desc));
    for (const s of e.sections ?? []) body.append(this.section(s.t, s.v));
    // 武器的正文就是 lore 第一行：拆开显示，不重复
    const lore = e.lore ?? '';
    const rest = (desc && lore.startsWith(desc) ? lore.slice(desc.length) : lore).trim();
    if (rest) body.append(para(hand.has('lore') ? 'lore ph' : 'lore', rest));
    return body;
  }

  private section(title: string, text: string): HTMLElement {
    const block = el('dl', 'wsec');
    block.append(Object.assign(el('dt'), { textContent: title }), Object.assign(el('dd'), { textContent: text }));
    return block;
  }

  private stats(e: WikiEntry): HTMLElement {
    const list = el('dl', 'kv');
    for (const s of e.stats ?? []) {
      const row = el('div', s.v.length > STAT_WIDE_VALUE || s.k.length > STAT_WIDE_KEY ? 'w' : '');
      row.append(Object.assign(el('dt'), { textContent: s.k }), Object.assign(el('dd', s.h ? 'ph' : ''), { textContent: s.v }));
      list.append(row);
    }
    return list;
  }

  private sourceLine(c: WikiCategory, text: string, isHand: boolean): HTMLElement {
    const line = el('p', 'wsrc');
    line.append(Object.assign(el('b'), { textContent: c.sourceLabel ?? WIKI_TEXT.sourceLabel }));
    line.append(isHand ? Object.assign(el('span', 'ph'), { textContent: text }) : text);
    return line;
  }

  /** 细滚动条：只在正文溢出时出现，跟着滚动 */
  private syncBar(): void {
    const { body, bar } = this;
    if (!body || !bar) return;
    const full = body.scrollHeight;
    const view = body.clientHeight;
    const more = full > view + 1;
    body.classList.toggle('dn', more && body.scrollTop + view < full - 1);
    bar.hidden = !more;
    if (!more) return;
    const thumb = Math.max(MIN_THUMB, (view * view) / full);
    bar.style.top = `${body.offsetTop}px`;
    bar.style.height = `${view}px`;
    const inner = bar.firstElementChild as HTMLElement;
    inner.style.height = `${thumb}px`;
    inner.style.transform = `translateY(${(body.scrollTop * (view - thumb)) / (full - view)}px)`;
  }
}
