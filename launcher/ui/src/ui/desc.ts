// 次级页右侧的说明面板：大图、标题、正文；配置文件名和键名只在开发模式显示
import { el } from './dom';

export interface Desc {
  img?: string;
  meta?: string;
  paragraphs: string[];
  lore?: string;
  /** 路径等数据小字 */
  data?: string[];
  /** 开发模式才显示 */
  dev?: string[];
  strip?: { src: string; on: boolean }[];
}

/** 数据小字只在「\」后折行 */
function dataLine(text: string): HTMLElement {
  const p = el('p', 'dt');
  text.split('\\').forEach((part, i) => {
    if (i > 0) p.append('\\', el('wbr'));
    p.append(part);
  });
  return p;
}

export function renderDesc(host: HTMLElement, title: string, d: Desc, dev: boolean): void {
  const head = el('div', 'dh');
  if (d.img) {
    const pic = el('div', 'pic');
    const img = el('img');
    img.alt = '';
    img.src = d.img;
    pic.append(img);
    head.append(pic);
  }
  const text = el('div');
  const h = el('h2');
  h.textContent = title;
  text.append(h);
  if (d.meta) text.append(Object.assign(el('p', 'meta'), { textContent: d.meta }));
  head.append(text);

  const parts: HTMLElement[] = [head, el('i', 'sep')];
  for (const line of d.paragraphs) parts.push(Object.assign(el('p'), { textContent: line }));
  if (d.strip) {
    const strip = el('div', 'strip');
    for (const s of d.strip) {
      const img = el('img', s.on ? 'on' : '');
      img.alt = '';
      img.src = s.src;
      strip.append(img);
    }
    parts.push(strip);
  }
  if (d.lore) parts.push(Object.assign(el('p', 'lore'), { textContent: d.lore }));
  for (const line of d.data ?? []) parts.push(dataLine(line));
  if (dev) for (const line of d.dev ?? []) parts.push(dataLine(line));
  host.replaceChildren(...parts);
}
