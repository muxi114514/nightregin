// 法环 PC 版按键提示：按键贴图 + 文字横排在底部；带 key 的做成按钮，点它等同按下该键
import { pressKey } from '../input/keys';

/**
 * icon 是 public\assets 下按键贴图的文件名（不带 .png），如 key_enter。
 * 一张贴图画了两个键时（如 key_lr），点贴图左半边按 alt，其余地方按 key
 */
export interface Hint { icon: string; label: string; key?: string; alt?: string }

export const keyIconUrl = (icon: string) => `${import.meta.env.BASE_URL}assets/${icon}.png`;

/** 建一条提示栏（.keys），点击按钮转成按键，按下时不抢焦点 */
export function createHintBar(): HTMLElement {
  const bar = document.createElement('div');
  bar.className = 'keys';
  bar.addEventListener('mousedown', (e) => e.preventDefault());
  bar.addEventListener('click', (e) => {
    // 双击的第二下不再执行（按钮不取焦点，不会有 Space 合成的 detail 0 点击）
    if (e.detail > 1) return;
    const b = e.target instanceof Element ? e.target.closest<HTMLElement>('[data-k]') : null;
    if (!b?.dataset.k || !bar.contains(b)) return;
    const img = b.querySelector('img')?.getBoundingClientRect();
    const left = !!b.dataset.alt && !!img && e.clientX < img.left + img.width / 2;
    pressKey(left ? b.dataset.alt! : b.dataset.k);
  });
  return bar;
}

export function setHints(bar: HTMLElement, hints: Hint[]): void {
  bar.replaceChildren(...hints.map((h) => {
    const el = document.createElement(h.key ? 'button' : 'span');
    if (h.key) {
      el.tabIndex = -1;
      el.dataset.k = h.key;
      if (h.alt) el.dataset.alt = h.alt;
    }
    const img = document.createElement('img');
    img.src = keyIconUrl(h.icon);
    img.alt = '';
    el.append(img, h.label);
    return el;
  }));
}
