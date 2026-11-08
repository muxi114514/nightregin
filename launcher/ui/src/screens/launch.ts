// 启动过场：标题菜单原地换成四步（预检 → 备份存档 → 载入配置 → 唤起游戏），当前步高亮。
// 画面只由 launch-view 驱动，谁发起的都一样（裁定 A）：失败弹失败横带（文案与动作见 texts.ts），
// 游戏仍在载入时问一次要不要结束进程（之后按键提示里一直留着入口），游戏窗口出现后渐黑并可切到游戏；
// 结束、取消或「返回」时退回菜单，并在 root 上发 LAUNCH_EXIT。
// root 是首页那一屏（.screen）
import '../styles/launch.css';
import type { LaunchView, Step } from '../api';
import type { AppCtx } from './app';
import { inputGuard } from '../input/guard';
import { openDialog, type DialogSpec } from '../ui/dialog';
import { createHintBar, keyIconUrl, setHints, type Hint } from '../ui/hints';
import { el } from '../ui/dom';
import { showToast, toastError } from '../ui/toast';
import { START_JOB, isStartAction, jobFor, runAction, type StartJob } from './actions';
import { runReport } from './report';
import {
  ACTION_TEXT, HINT_TEXT, RUNNING_TEXT, STEP_TEXT, UNKNOWN_FAILURE_TEXT, WAKE_TEXT, failureCopy, type ActionId, type Copy,
} from './texts';

/** 退回菜单时在 root 上发出的事件（首页据此刷新） */
export const LAUNCH_EXIT = 'launch-exit';

export interface LaunchStatus {
  /** 启动进行中（step / waking），× 要先确认 */
  inProgress: boolean;
}

const STEPS: Step[] = ['preflight', 'backup', 'loadConfig', 'wake'];
const ICON = { idle: 'chk_off', run: 'chk_off', ok: 'chk_on', err: 'chk_fail' } as const;
type StepState = keyof typeof ICON;
type Failed = Extract<LaunchView, { phase: 'failed' }>;

const IDLE: LaunchView = { phase: 'idle' };
/** 启动还在进行：还没到游戏窗口、也没有结局 */
export const isLaunchBusy = (v: LaunchView) => v.phase === 'step' || v.phase === 'waking';

class LaunchScreen {
  private readonly root: HTMLElement;
  private readonly ctx: AppCtx;
  private readonly steps: HTMLOListElement;
  private readonly rows: { li: HTMLLIElement; img: HTMLImageElement; d: HTMLElement }[];
  private readonly msg: HTMLElement;
  private readonly msgText: HTMLElement;
  private readonly keys: HTMLElement;
  private readonly veil: HTMLElement;
  private view: LaunchView = IDLE;
  private shown = false;
  /** 每来一个进度加一；异步动作回来时据此判断画面是否已被新进度取代 */
  private seq = 0;
  /** 本画面开的对话框（失败横带 / 仍在载入），新进度来时撤下 */
  private dialog: AbortController | null = null;
  /** 「游戏仍在载入」已自动问过；之后只从按键提示再打开，不随 Waking 重开 */
  private slowAsked = false;
  /** 取消已受理，等 cancelled */
  private cancelling = false;
  /** 这次连续重试用的参数：回到首页时清掉 */
  private job: StartJob = START_JOB.retry;

  constructor(root: HTMLElement, ctx: AppCtx) {
    this.root = root;
    this.ctx = ctx;
    this.steps = el('ol', 'steps');
    this.steps.setAttribute('aria-live', 'polite');
    this.rows = STEPS.map((s) => {
      const li = el('li', 'step');
      const img = el('img', 'loop');
      img.alt = '';
      const n = el('span', 'n');
      n.textContent = STEP_TEXT[s];
      const d = el('span', 'd');
      li.append(img, n, d);
      return { li, img, d };
    });
    this.steps.append(...this.rows.map((r) => r.li));
    this.msg = el('p', 'lmsg');
    this.msgText = el('span', 'brz');
    this.msg.append(this.msgText);
    this.keys = createHintBar();
    this.keys.classList.add('lkeys');
    // 渐黑画面：一行「游戏运行中」+ 可点的「切到游戏」（Enter）
    this.veil = el('div', 'veil');
    const line = el('p', 'vmsg');
    line.textContent = RUNNING_TEXT;
    const veilKeys = createHintBar();
    veilKeys.classList.add('vkeys');
    setHints(veilKeys, [{ icon: 'key_enter', label: ACTION_TEXT.focusGame, key: 'Enter' }]);
    this.veil.append(line, veilKeys);
    root.append(this.steps, this.msg, this.keys, this.veil);
    this.paintAll('idle');
  }

  get status(): LaunchStatus {
    return { inProgress: this.shown && isLaunchBusy(this.view) };
  }

  apply(v: LaunchView): void {
    if (!v || typeof v !== 'object') return;
    this.seq++;
    const prev = this.view;
    switch (v.phase) {
      case 'step':
        this.view = v;
        return this.progress(prev, v.step, false, false);
      case 'waking':
        this.view = v;
        return this.progress(prev, 'wake', !!v.slow, !!v.gameLoading);
      case 'running':
        this.view = v;
        return this.running();
      case 'failed':
        this.view = v;
        return this.failed(prev, v);
      case 'cancelled':
      case 'finished':
        return this.exit();
      case 'idle':
        return;
      default:
        console.warn('不认识的启动进度', v);
    }
  }

  /** 启动画面上的按键：唤起前 Esc 取消；仍在载入时 Enter 打开结束进程的提问；渐黑时 Enter 切到游戏；其余吞掉 */
  key(k: string): boolean {
    if (!this.shown) return false;
    const v = this.view;
    if (k === 'Escape' && this.beforeWake() && !this.cancelling) this.cancel();
    else if (k === 'Enter' && v.phase === 'waking' && v.slow) this.askSlow();
    else if (k === 'Enter' && v.phase === 'running') this.ctx.api.focusGame().catch(toastError);
    return true;
  }

  private beforeWake(): boolean {
    return this.shown && this.view.phase === 'step' && this.view.step !== 'wake';
  }

  /** 取消被受理后，Esc 提示换成「正在取消…」直到 cancelled */
  private cancel(): void {
    this.ctx.api.cancelLaunch()
      .then((ok) => {
        if (!ok || !this.beforeWake()) return;
        this.cancelling = true;
        this.paintKeys();
      })
      .catch(toastError);
  }

  /** 新的一次启动（之前没显示、或上一次已失败 / 结束）：从头画 */
  private begin(prev: LaunchView): void {
    if (this.shown && isLaunchBusy(prev)) return;
    this.closeDialog();
    // 菜单原地换成过场也算切页：之后 500ms 内的点击丢弃
    if (!this.shown) inputGuard.markLayerChange(performance.now());
    this.shown = true;
    this.slowAsked = false;
    this.cancelling = false;
    this.root.dataset.mode = 'launch';
    this.steps.classList.remove('fail');
    this.veil.classList.remove('on');
    this.paintAll('idle');
  }

  private progress(prev: LaunchView, step: Step, slow: boolean, loading: boolean): void {
    this.begin(prev);
    const at = STEPS.indexOf(step);
    this.rows.forEach((_, k) => this.paint(k, k < at ? 'ok' : k === at ? 'run' : 'idle'));
    this.setMsg(slow ? WAKE_TEXT.slow : loading ? WAKE_TEXT.loading : '');
    this.paintKeys();
    // 只在第一次变慢时自动问；之后的 Waking 不重开、也不关掉开着的提问
    if (slow && !this.slowAsked) {
      this.slowAsked = true;
      this.askSlow();
    }
  }

  private askSlow(): void {
    this.ask({ msg: WAKE_TEXT.slow, opts: [ACTION_TEXT.collectReport, WAKE_TEXT.endProcesses, WAKE_TEXT.keepWaiting], def: 2, esc: 2 }, (i) => {
      if (i === 0) void runReport(this.ctx);
      if (i === 1) this.ctx.api.endProcesses().catch(toastError);
    });
  }

  private running(): void {
    if (!this.shown) this.begin(IDLE);
    this.closeDialog();
    this.paintAll('ok');
    this.setMsg('');
    this.paintKeys();
    this.veil.classList.add('on');
  }

  private failed(prev: LaunchView, v: Failed): void {
    const copy = failureCopy(v.reason, this.ctx.devMode());
    if (!copy) {
      console.error('不认识的失败原因，回到首页', v.reason);
      showToast(UNKNOWN_FAILURE_TEXT);
      this.exit();
      return;
    }
    this.begin(prev);
    const at = STEPS.indexOf(v.step);
    this.rows.forEach((_, k) => this.paint(k, k < at ? 'ok' : k === at ? 'err' : 'idle', k === at ? copy.short : ''));
    this.steps.classList.add('fail');
    this.setMsg('');
    this.paintKeys();
    this.askFailure(copy);
  }

  /** 失败横带：贴在上方、带失败图标、藏标志；「返回」总在最后，Esc 选它 */
  private askFailure(copy: Copy): void {
    const actions: ActionId[] = copy.actions.length ? copy.actions : ['back'];
    const seq = this.seq;
    const spec = { msg: copy.text, opts: actions.map((a) => ACTION_TEXT[a]), def: copy.actions.length ? copy.def : 0 };
    this.ask({ ...spec, icon: true, top: true, fail: true }, (i) => this.act(actions[i], seq, copy));
  }

  /** 失败横带的动作：重新开始的等新进度切画面，被拒（如已在启动中）只提示并留在失败画面（裁定 A）；其余回首页 */
  private act(a: ActionId, seq: number, copy: Copy): void {
    const call = runAction(this.ctx, a, this.job);
    if (a === 'collectReport') {
      // 整理完留在失败画面，还可以重试
      void call?.finally(() => {
        if (this.seq === seq) this.askFailure(copy);
      });
      return;
    }
    if (isStartAction(a) || a === 'alignAndRetry') {
      if (isStartAction(a)) this.job = jobFor(a, this.job);
      call?.catch((e) => {
        toastError(e);
        if (this.seq === seq) this.askFailure(copy);
      });
      return;
    }
    if (a === 'chooseDir') {
      // 选完目录再回首页（首页随之刷新）
      call?.catch(toastError).finally(() => {
        if (this.seq === seq) this.exit();
      });
      return;
    }
    call?.catch(toastError);
    this.exit();
  }

  /** 退回首页菜单 */
  private exit(): void {
    if (this.shown) inputGuard.markLayerChange(performance.now());
    this.closeDialog();
    this.view = IDLE;
    this.shown = false;
    this.job = START_JOB.retry;
    this.cancelling = false;
    this.root.dataset.mode = 'menu';
    this.steps.classList.remove('fail');
    this.veil.classList.remove('on');
    this.setMsg('');
    this.paintKeys();
    this.root.dispatchEvent(new CustomEvent(LAUNCH_EXIT));
  }

  private ask(spec: DialogSpec, then: (i: number) => void): void {
    this.closeDialog();
    const ctl = new AbortController();
    this.dialog = ctl;
    void openDialog(spec, ctl.signal).then((i) => {
      if (this.dialog === ctl) this.dialog = null;
      if (i < 0 || ctl.signal.aborted) return;
      try {
        then(i);
      } catch (err) {
        console.error('启动画面的动作出错', err);
      }
    });
  }

  private closeDialog(): void {
    this.dialog?.abort();
    this.dialog = null;
  }

  private paintAll(state: StepState): void {
    this.rows.forEach((_, k) => this.paint(k, state));
  }

  private paint(k: number, state: StepState, detail = ''): void {
    const r = this.rows[k];
    r.li.className = state === 'idle' ? 'step' : `step ${state}`;
    const src = keyIconUrl(ICON[state]);
    if (r.img.getAttribute('src') !== src) r.img.src = src;
    r.d.textContent = detail;
  }

  /** 唤起那行字；清空时保留旧字，让它跟着淡出 */
  private setMsg(text: string): void {
    if (text) this.msgText.textContent = text;
    this.msg.classList.toggle('on', !!text);
  }

  /**
   * 按键提示由当前状态生成（每个进度重画都不会丢）：唤起前可点的 Esc「取消启动」，受理后「正在取消…」；
   * 唤起中只有说明；仍在载入时一直留着可点的「结束 me3 和游戏进程」（Enter）
   */
  private paintKeys(): void {
    const v = this.view;
    const hints: Hint[] = [];
    let note = '';
    if (!this.shown || (v.phase === 'step' && !STEPS.includes(v.step))) {
      // 不显示或数据不对：不给提示
    } else if (this.beforeWake()) {
      if (this.cancelling) note = HINT_TEXT.cancelling;
      else {
        hints.push({ icon: 'key_esc', label: HINT_TEXT.cancelLaunch, key: 'Escape' });
        note = HINT_TEXT.cancelNote;
      }
    } else if (v.phase === 'waking' && v.slow) {
      hints.push({ icon: 'key_enter', label: WAKE_TEXT.endProcesses, key: 'Enter' });
    } else if (isLaunchBusy(v)) {
      note = HINT_TEXT.waking;
    }
    setHints(this.keys, hints);
    if (note) this.keys.append(Object.assign(document.createElement('span'), { textContent: note }));
  }
}

const screens = new WeakMap<HTMLElement, LaunchScreen>();

/** 预建启动过场的节点（首页挂载时调），第一次进入时淡入照常 */
export function initLaunch(root: HTMLElement, ctx: AppCtx): void {
  if (!screens.has(root)) screens.set(root, new LaunchScreen(root, ctx));
}

/** 把一个启动进度画到 root 上 */
export function runLaunch(root: HTMLElement, view: LaunchView): void {
  try {
    screens.get(root)?.apply(view);
  } catch (err) {
    console.error('启动画面出错', err);
  }
}

/** 启动画面显示着时由它处理按键，返回 true；否则 false，交给首页菜单 */
export function launchKey(root: HTMLElement, key: string): boolean {
  return screens.get(root)?.key(key) ?? false;
}

export function launchStatus(root: HTMLElement): LaunchStatus {
  return screens.get(root)?.status ?? { inProgress: false };
}
