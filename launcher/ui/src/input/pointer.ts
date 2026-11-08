// 鼠标：指针坐标真的变了才算悬停（切页、重绘、动画后浏览器给静止鼠标补发的移动不算）；
// 右键 = 返回且不弹菜单；侧键（mouseup 的 button 3 / 4，浏览器前进后退）一律阻止；
// 切页、开关对话框后的点击按 InputGuard 丢弃，窗口按钮（.win）除外
import { inputGuard } from './guard';
import { pressKey } from './keys';

export type HoverHandler = (target: Element) => void;

const hovers = new Set<HoverHandler>();
let lastX = Number.NaN;
let lastY = Number.NaN;
let installed = false;

/** 订阅真实的悬停，返回取消订阅 */
export function onRealHover(cb: HoverHandler): () => void {
  hovers.add(cb);
  return () => hovers.delete(cb);
}

function onMove(e: PointerEvent): void {
  if (e.clientX === lastX && e.clientY === lastY) return;
  lastX = e.clientX;
  lastY = e.clientY;
  if (!(e.target instanceof Element)) return;
  for (const cb of hovers) {
    try {
      cb(e.target);
    } catch (err) {
      console.error('悬停处理出错', err);
    }
  }
}

function onClick(e: MouseEvent): void {
  if (e.target instanceof Element && e.target.closest('.win')) return;
  if (!inputGuard.clickAllowed(performance.now())) {
    e.preventDefault();
    e.stopImmediatePropagation();
  }
}

/** 装全局监听，重复调用无效 */
export function installPointer(): void {
  if (installed) return;
  installed = true;
  addEventListener('pointermove', onMove, true);
  addEventListener('pointerdown', () => inputGuard.markPointerDown(performance.now()), true);
  addEventListener('click', onClick, true);
  addEventListener('contextmenu', (e) => {
    e.preventDefault();
    pressKey('Escape');
  }, true);
  // 右键不抢焦点（输入中右键 = 取消输入）
  addEventListener('mousedown', (e) => {
    if (e.button === 2) e.preventDefault();
  }, true);
  addEventListener('mouseup', (e) => {
    if (e.button === 3 || e.button === 4) e.preventDefault();
  }, true);
}
