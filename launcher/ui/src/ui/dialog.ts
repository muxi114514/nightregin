// 对话框：全宽 Dialog 横带（草图 v4 的 openDlg）。打开时下层 .screen 设 inert，按键和悬停只归对话框；
// ←→ 选择、Enter 确定、Esc 选 esc 那一项，单击选项直接执行（开关后 500ms 内的点击由 pointer 丢弃）。
// 可以叠放：上层关掉后回到下层那个
import { inputGuard } from '../input/guard';
import { pushKeyHandler } from '../input/keys';
import { onRealHover } from '../input/pointer';
import { createHintBar, setHints, type Hint } from './hints';

export interface DialogSpec {
  msg: string;
  sub?: string;
  /** 路径等数据小字，只在「\」后折行 */
  dt?: string;
  opts: string[];
  /** 默认选中项，默认 0 */
  def?: number;
  /** Esc 对应项，默认最后一项 */
  esc?: number;
  /** Esc 和右键无效（必须做完这一步） */
  noEsc?: boolean;
  /** 贴在上方，不挡住下面的内容 */
  top?: boolean;
  /** 启动失败：舞台加 .mfail（藏标志，失败那一步留在遮罩之上） */
  fail?: boolean;
  /** 显示失败图标 */
  icon?: boolean;
}

const HINT = { select: '选择', ok: '确定' };

interface Shown { spec: DialogSpec; sel: number; esc: number; resolve: (i: number) => void }
interface Parts { root: HTMLElement; icon: HTMLElement; msg: HTMLElement; sub: HTMLElement; dt: HTMLElement; opts: HTMLElement; keys: HTMLElement }

const stack: Shown[] = [];
let parts: Parts | null = null;
let releaseKeys: (() => void) | null = null;
let releaseHover: (() => void) | null = null;
let inerted: HTMLElement[] = [];

const host = () => document.getElementById('stage') ?? document.body;
const clamp = (i: number, n: number) => Math.max(0, Math.min(n - 1, i));
const top = () => stack[stack.length - 1];

export function isDialogOpen(): boolean {
  return stack.length > 0;
}

/** 打开对话框，返回选中项的下标；signal 中止时撤下它（不必在栈顶），返回 -1 */
export function openDialog(spec: DialogSpec, signal?: AbortSignal): Promise<number> {
  if (signal?.aborted) return Promise.resolve(-1);
  const n = Math.max(1, spec.opts.length);
  const opts = spec.opts.length ? spec.opts : [HINT.ok];
  return new Promise((resolve) => {
    const shown: Shown = { spec: { ...spec, opts }, sel: clamp(spec.def ?? 0, n), esc: clamp(spec.esc ?? n - 1, n), resolve };
    stack.push(shown);
    if (stack.length === 1) attach();
    render();
    const now = performance.now();
    inputGuard.markLayerChange(now);
    inputGuard.markSublayerEnter(now);
    signal?.addEventListener('abort', () => dismiss(shown), { once: true });
  });
}

/** 确认框；destructive 时默认选中「取消」，Esc 总是取消；被 signal 撤下算取消 */
export async function confirm(
  o: { text: string; ok: string; cancel: string; destructive: boolean },
  signal?: AbortSignal,
): Promise<boolean> {
  const i = await openDialog({ msg: o.text, opts: [o.ok, o.cancel], def: o.destructive ? 1 : 0, esc: 1 }, signal);
  return i === 0;
}

/** 外部状态变了、对话框已过时：撤下并以 -1 返回；在栈顶时同关闭 */
function dismiss(shown: Shown): void {
  const i = stack.indexOf(shown);
  if (i < 0) return;
  if (i === stack.length - 1) {
    close(-1);
    return;
  }
  stack.splice(i, 1);
  shown.resolve(-1);
}

function close(i: number): void {
  const shown = stack.pop();
  if (!shown) return;
  if (stack.length) render();
  else detach();
  inputGuard.markLayerChange(performance.now());
  shown.resolve(i);
}

function build(): Parts {
  const root = document.createElement('div');
  root.className = 'dlg';
  root.setAttribute('role', 'dialog');
  root.setAttribute('aria-modal', 'true');
  const box = document.createElement('div');
  box.className = 'dbox band';
  const el = (tag: string, cls: string) => Object.assign(document.createElement(tag), { className: cls });
  const p: Parts = {
    root, icon: el('i', 'dico'), msg: el('p', 'msg'), sub: el('p', 'sub'), dt: el('p', 'dt'), opts: el('div', 'opts'),
    keys: createHintBar(),
  };
  box.append(p.icon, p.msg, p.sub, p.dt, p.opts);
  root.append(box, p.keys);
  p.opts.addEventListener('click', (e) => {
    const b = e.target instanceof Element ? e.target.closest<HTMLElement>('.opt') : null;
    if (b && top()) close(Number(b.dataset.i));
  });
  return p;
}

/** 第一个对话框打开：接管按键与悬停，下层设 inert */
function attach(): void {
  parts ??= build();
  const stage = host();
  // 舞台重建过时重新挂上
  if (parts.root.parentElement !== stage) stage.append(parts.root);
  inerted = [...stage.querySelectorAll<HTMLElement>(':scope > .screen')].filter((s) => !s.inert);
  inerted.forEach((s) => { s.inert = true; });
  // 焦点留在下层（如输入框）会挡住 Enter
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  stage.classList.add('modal');
  parts.root.classList.add('on');
  parts.root.removeAttribute('aria-hidden');
  releaseKeys = pushKeyHandler(onKey);
  releaseHover = onRealHover((t) => {
    const b = t.closest<HTMLElement>('.opt');
    if (b && parts?.opts.contains(b)) pick(Number(b.dataset.i));
  });
}

function detach(): void {
  releaseKeys?.();
  releaseHover?.();
  releaseKeys = releaseHover = null;
  inerted.forEach((s) => { s.inert = false; });
  inerted = [];
  host().classList.remove('modal', 'mfail');
  parts?.root.classList.remove('on', 'top');
  parts?.root.setAttribute('aria-hidden', 'true');
}

function onKey(k: string): boolean {
  const d = top();
  if (!d) return false;
  if (k === 'ArrowLeft' || k === 'ArrowUp') pick(d.sel - 1);
  else if (k === 'ArrowRight' || k === 'ArrowDown') pick(d.sel + 1);
  else if (k === 'Enter') close(d.sel);
  else if (k === 'Escape' && !d.spec.noEsc) close(d.esc);
  else if (k === 'Escape') return true;
  else return false;
  return true;
}

function pick(i: number): void {
  const d = top();
  if (!d || !parts) return;
  d.sel = clamp(i, d.spec.opts.length);
  [...parts.opts.children].forEach((b, k) => b.classList.toggle('sel', k === d.sel));
}

/** 路径只在「\」后折行 */
function setBreakable(el: HTMLElement, text: string): void {
  el.replaceChildren();
  text.split('\\').forEach((part, i, all) => {
    el.append(part);
    if (i < all.length - 1) el.append('\\', document.createElement('wbr'));
  });
}

function render(): void {
  const d = top();
  if (!d || !parts) return;
  const { spec } = d;
  parts.root.classList.toggle('top', !!spec.top);
  host().classList.toggle('mfail', !!spec.fail);
  parts.icon.hidden = !spec.icon;
  parts.msg.textContent = spec.msg;
  parts.sub.textContent = spec.sub ?? '';
  setBreakable(parts.dt, spec.dt ?? '');
  parts.opts.replaceChildren(...spec.opts.map((label, i) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'opt';
    b.tabIndex = -1;
    b.dataset.i = String(i);
    const span = document.createElement('span');
    span.textContent = label;
    b.append(span);
    return b;
  }));
  const many = spec.opts.length > 1;
  const hints: Hint[] = [
    // ←→ 也能点：点贴图左半边同 ←，其余同 →
    ...(many ? [{ icon: 'key_lr', label: HINT.select, key: 'ArrowRight', alt: 'ArrowLeft' }] : []),
    { icon: 'key_enter', label: HINT.ok, key: 'Enter' },
    ...(many ? [{ icon: 'key_esc', label: spec.opts[d.esc], key: 'Escape' }] : []),
  ];
  setHints(parts.keys, hints);
  pick(d.sel);
}
