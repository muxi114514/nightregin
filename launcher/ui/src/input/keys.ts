// 统一的键盘处理：屏蔽浏览器快捷键和 Ctrl+滚轮；其余按键只交给栈顶的处理器（对话框压在当前页之上）。
// Enter 按住产生的重复、进入子层 300ms 内都不执行；焦点在输入框时按键交给输入框，只有 Esc 继续分发
import { inputGuard } from './guard';

/** 返回 true 表示处理了（阻止默认行为） */
export type KeyHandler = (key: string) => boolean;

const stack: KeyHandler[] = [];
let installed = false;

/** 压入一层，返回弹出函数；只有栈顶收到按键，不往下层传 */
export function pushKeyHandler(h: KeyHandler): () => void {
  stack.push(h);
  return () => {
    const i = stack.lastIndexOf(h);
    if (i >= 0) stack.splice(i, 1);
  };
}

/** 按键提示的点击、右键返回：和真实按键走同一条分发 */
export function pressKey(key: string): boolean {
  return dispatch(key, false);
}

function dispatch(key: string, repeat: boolean): boolean {
  if (key === 'Enter' && !inputGuard.enterAllowed(performance.now(), repeat)) return true;
  const top = stack[stack.length - 1];
  if (!top) return false;
  try {
    return top(key);
  } catch (err) {
    console.error('按键处理出错', err);
    return true;
  }
}

/** 刷新、查找、打印、保存、源码、下载、开发者工具、光标浏览、缩放、前进后退和 Tab 遍历焦点 */
export function isBrowserShortcut(e: KeyboardEvent): boolean {
  const k = e.key.toLowerCase();
  const ctrl = e.ctrlKey || e.metaKey;
  if (k === 'tab' || k === 'f3' || k === 'f5' || k === 'f7' || k === 'f12') return true;
  if (ctrl && k.length === 1 && 'rfgpsuj+-=0'.includes(k)) return true;
  if (ctrl && e.shiftKey && k === 'i') return true;
  return e.altKey && (k === 'arrowleft' || k === 'arrowright');
}

function isEditable(t: EventTarget | null): boolean {
  return t instanceof HTMLElement
    && (t.isContentEditable || t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || t instanceof HTMLSelectElement);
}

function onKeyDown(e: KeyboardEvent): void {
  // 输入法组字中的按键（拼音选字的 Enter、方向键）交给输入法
  if (e.defaultPrevented || e.isComposing || e.keyCode === 229) return;
  if (isEditable(e.target) && e.key !== 'Escape') return;
  if (dispatch(e.key, e.repeat)) e.preventDefault();
}

/** 装全局监听，重复调用无效 */
export function installKeys(): void {
  if (installed) return;
  installed = true;
  // 捕获阶段最先拦，焦点在哪都一样
  addEventListener('keydown', (e) => {
    if (isBrowserShortcut(e)) {
      e.preventDefault();
      e.stopImmediatePropagation();
    }
  }, true);
  addEventListener('keydown', onKeyDown);
  addEventListener('wheel', (e) => {
    if (e.ctrlKey) e.preventDefault();
  }, { passive: false });
}
