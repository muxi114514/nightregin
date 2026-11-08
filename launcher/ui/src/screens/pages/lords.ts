// 永夜王选择：法环物品栏式头像格。Enter / 单击切换，R 全部选中或取消，F 跳到图鉴，Esc 回到「永夜王」那一行
import { onRealHover } from '../../input/pointer';
import { asset, el } from '../../ui/dom';
import type { SysHost } from '../sys';

export const LORD_COUNT = 8;
/** 下标同 ChanceSetter 的 8 位永夜夜王：[官方中文名, 社区叫法] */
export const LORDS: [string, string][] = [
  ['格拉狄乌斯', '三狼'], ['艾德雷', '大嘴'], ['格诺斯塔', '蝴蝶与蝎子'], ['玛利斯', '水母'],
  ['利普拉', '山羊'], ['弗格尔', '人马'], ['卡莉果', '冰龙'], ['哈尔莫妮亚', '女武神'],
];
/** 头像文件按 NightBossMenuParam 行号：哈尔莫妮亚是第 8 行 */
const IMAGE_ROW = [0, 1, 2, 3, 4, 5, 6, 8];
export const lordImage = (i: number, ever: boolean) => asset(`lords/nl_${IMAGE_ROW[i]}${ever ? '_ever' : ''}.webp`);
/** 这一位在图鉴里的条目 id */
export const lordWikiId = (i: number) => `nightlord-${IMAGE_ROW[i]}`;

/** 每行 4 格 */
const FIRST_ROW = 4;

export interface LordsGrid {
  el: HTMLElement;
  open(focus?: number): void;
  handleKey(key: string): boolean;
  close(): void;
}

export function createLordsGrid(o: {
  host(): SysHost;
  list(): number[];
  onChange(list: number[]): void;
  onClose(): void;
  openWiki(focus: number): void;
  dev(): boolean;
}): LordsGrid {
  const root = el('div');
  root.style.display = 'contents';
  let focus = 0;
  let opened = false;

  const cells = LORDS.map(([name], i) => {
    const b = el('button', 'lord');
    b.type = 'button';
    b.tabIndex = -1;
    b.dataset.i = String(i);
    b.setAttribute('aria-label', name);
    const part = (cls: string, src: string) => {
      const img = el('img', cls);
      img.alt = '';
      img.src = src;
      return img;
    };
    b.append(part('fr n', asset('slot.png')), part('fr l', asset('slot_lord.png')), part('pt', lordImage(i, false)), part('cur', asset('sl_cursor.png')));
    return b;
  });
  root.append(...cells);
  root.addEventListener('click', (e) => {
    const b = e.target instanceof Element ? e.target.closest<HTMLElement>('.lord') : null;
    if (!b || e.detail > 1) return;
    focus = Number(b.dataset.i);
    toggle();
  });
  onRealHover((t) => {
    const b = opened ? t.closest<HTMLElement>('.lord') : null;
    if (b && root.contains(b) && Number(b.dataset.i) !== focus) {
      focus = Number(b.dataset.i);
      paint();
    }
  });

  function paint(): void {
    const list = o.list();
    const on = list.includes(focus);
    cells.forEach((c, i) => {
      const lit = list.includes(i);
      c.classList.toggle('on', lit);
      c.classList.toggle('f', i === focus);
      c.setAttribute('aria-pressed', String(lit));
      const pt = c.querySelector<HTMLImageElement>('.pt');
      const src = lordImage(i, lit);
      if (pt && pt.getAttribute('src') !== src) pt.src = src;
    });
    const host = o.host();
    host.setTitle('永夜王', `${list.length} / ${LORD_COUNT}`);
    host.setDesc(LORDS[focus][0], {
      img: lordImage(focus, on),
      meta: `${LORDS[focus][1]} · 编号 ${focus}`,
      paragraphs: [
        on ? '以永夜王形态出现，比普通形态更强。' : '以普通形态出现。',
        on ? '按 Enter 或单击，改回普通形态。' : '按 Enter 或单击，让它以永夜王形态出现。',
      ],
      lore: '夜色最浓之时，王者不再收敛。',
      dev: ['dll\\ChanceSetter.ini', '[everdarkChances.depth1～5] 选中 100 / 未选 0'],
    });
    host.setHints([
      { icon: 'key_udlr', label: '选择' },
      { icon: 'key_enter', label: on ? '改回普通' : '设为永夜', key: 'Enter' },
      { icon: 'key_r', label: list.length === LORD_COUNT ? '全部取消' : '全部选中', key: 'r' },
      { icon: 'key_f', label: '图鉴', key: 'f' },
      { icon: 'key_esc', label: '返回', key: 'Escape' },
    ]);
  }

  function change(list: number[]): void {
    o.onChange(list);
    paint();
  }

  function toggle(): void {
    const list = o.list();
    change(list.includes(focus) ? list.filter((x) => x !== focus) : [...list, focus].sort((a, b) => a - b));
  }

  /** 左右不换行，上下在两行之间对齐 */
  function move(key: string): void {
    const second = focus >= FIRST_ROW;
    const col = second ? focus - FIRST_ROW : focus;
    let next = focus;
    if (key === 'ArrowLeft' && col > 0) next--;
    else if (key === 'ArrowRight' && col < (second ? LORD_COUNT - FIRST_ROW - 1 : FIRST_ROW - 1)) next++;
    else if (key === 'ArrowUp' && second) next = col;
    else if (key === 'ArrowDown' && !second) next = FIRST_ROW + Math.min(col, LORD_COUNT - FIRST_ROW - 1);
    if (next !== focus) {
      focus = next;
      paint();
    }
  }

  return {
    el: root,
    open(at) {
      if (at !== undefined) focus = at;
      opened = true;
      o.host().enterSub();
      paint();
    },
    close() {
      opened = false;
      o.onClose();
    },
    handleKey(key) {
      if (key === 'Escape') {
        this.close();
      } else if (key === 'Enter') {
        toggle();
      } else if (key.toLowerCase() === 'f') {
        opened = false;
        o.openWiki(focus);
      } else if (key.toLowerCase() === 'r') {
        change(o.list().length === LORD_COUNT ? [] : LORDS.map((_, i) => i));
      } else {
        move(key);
      }
      return true;
    },
  };
}
