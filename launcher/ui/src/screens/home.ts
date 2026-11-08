// 首页（标题画面，照草图 v4）：黄金树标志、六项标题菜单（更新在计划 4）、
// 状态行三段（版本、深夜、就绪，后两段可点）、问题横带（只显示 issues[0]）、可点的「确定」提示。
// 背景、窗口按钮和事件订阅在 app.ts；启动进度交给 launch.ts，菜单原地换成启动过场
import '../styles/home.css';
import type { HomeState, IssueKind } from '../api';
import { pushKeyHandler } from '../input/keys';
import { onRealHover } from '../input/pointer';
import { Banner } from '../ui/banner';
import { isDialogOpen } from '../ui/dialog';
import { asset, el } from '../ui/dom';
import { createHintBar, setHints } from '../ui/hints';
import { Menu } from '../ui/menu';
import { toastError } from '../ui/toast';
import { START_JOB, runAction } from './actions';
import type { AppCtx, Route } from './app';
import type { SysTab } from './sys';
import { LAUNCH_EXIT, initLaunch, launchKey, launchStatus } from './launch';
import {
  ACTION_TEXT, HINT_TEXT, LOGO_ALT, MENU_TEXT, STATUS_TEXT, issueCopy, type ActionId, type Copy,
} from './texts';

/** 菜单第几项打开哪个次级页 */
const MENU_TABS: Record<number, SysTab> = { 2: 'night', 3: 'options', 4: 'saves' };
/** 图鉴在菜单里的位置 */
const WIKI_ROW = 1;

/** 状态行的问题段跳到能处理它的那一行 */
function routeOfIssue(kind: IssueKind): Route {
  switch (kind) {
    case 'gameNotFound': return { page: 'sys', tab: 'options', row: 'gameDir' };
    case 'saveBusy':
    case 'suffixMismatch':
    case 'chanceSetterSuffix': return { page: 'sys', tab: 'saves', row: 'suffix' };
    case 'everdarkMismatch': return { page: 'sys', tab: 'night', row: 'lords' };
    case 'lastExitAbnormal': return { page: 'sys', tab: 'options', row: 'report' };
    default: return { page: 'sys', tab: 'options', row: 'about' };
  }
}

export class HomeScreen {
  readonly section: HTMLElement;
  private readonly ctx: AppCtx;
  private readonly menu: Menu;
  private readonly banner = new Banner();
  private readonly status: HTMLElement;
  /** 鼠标上一次在菜单（含横带动作）里 */
  private pointerIn = false;
  /** 只服务单击执行之后的光标位置：鼠标移开时光标留在单击过的项上，不回到「开始游戏」；-1 为没有 */
  private pinned = -1;

  constructor(ctx: AppCtx) {
    this.ctx = ctx;
    this.section = el('section', 'screen on');
    this.section.id = 'home';
    this.section.dataset.mode = 'menu';
    const logo = el('img', 'logo');
    logo.src = asset('logo.webp');
    logo.alt = LOGO_ALT;
    this.menu = new Menu({
      items: MENU_TEXT.map((label, i) => ({ label, disabled: i !== 0 && i !== WIKI_ROW && !Object.hasOwn(MENU_TABS, i) })),
      // 开始游戏：画面等 launch-view 再切（裁定 A）；被拒（如已在启动中）只提示，留在原处
      onActivate: (i) => {
        if (i === 0) ctx.api.startGame(START_JOB.retry).catch(toastError);
        else if (i === WIKI_ROW) ctx.go({ page: 'wiki' });
        else if (Object.hasOwn(MENU_TABS, i)) ctx.go({ page: 'sys', tab: MENU_TABS[i] });
      },
    });
    this.status = el('nav', 'status');
    const keys = createHintBar();
    keys.classList.add('hkeys');
    setHints(keys, [{ icon: 'key_enter', label: HINT_TEXT.ok, key: 'Enter' }]);
    // 菜单这一层整体淡出换成启动过场；子元素的入场动画不受影响
    const front = el('div', 'hmenu');
    front.append(this.menu.el, this.banner.el, this.status, keys);
    this.section.append(logo, front);
    initLaunch(this.section, ctx);
    this.wire();
  }

  /** 回到首页：光标放到 row（默认「开始游戏」） */
  show(row: number): void {
    this.pinned = -1;
    this.pointerIn = false;
    this.menu.select(row);
  }

  launchInProgress(): boolean {
    return launchStatus(this.section).inProgress;
  }

  private wire(): void {
    // 只有栈底这一层：启动画面显示着时交给它，否则给菜单；对话框总压在上面
    pushKeyHandler((k) => launchKey(this.section, k) || this.menu.handleKey(k));
    onRealHover((t) => this.onHover(t));
    this.menu.el.addEventListener('click', (e) => this.pinClicked(e.target, false));
    this.banner.el.addEventListener('click', (e) => this.pinClicked(e.target, true));
    this.section.addEventListener(LAUNCH_EXIT, () => this.ctx.go({ page: 'home', row: 0 }));
  }

  render(h: HomeState): void {
    const issues = Array.isArray(h.issues) ? h.issues : [];
    const top = issues.length ? issueCopy(issues[0], this.ctx.devMode()) : null;
    this.paintStatus(h, issues.length === 0, top);
    this.paintBanner(top);
  }

  /** 状态行：版本（计划 4 接）、深夜、就绪或问题；后两段单击跳到对应页 */
  private paintStatus(h: HomeState, ready: boolean, top: Copy | null): void {
    const segs: { text: string; to?: Route }[] = [
      { text: typeof h.version === 'string' && h.version ? h.version : STATUS_TEXT.versionUnknown },
    ];
    const night: Route = { page: 'sys', tab: 'night' };
    const dn = h.deepNight;
    // nighter.json 读不出来（null）时不写深夜段
    if (dn && typeof dn === 'object') {
      if (!dn.enabled) segs.push({ text: STATUS_TEXT.deepNightOff, to: night });
      else if (Number.isFinite(dn.level)) segs.push({ text: STATUS_TEXT.deepNight(dn.level), to: night });
    }
    if (ready) segs.push({ text: STATUS_TEXT.ready, to: { page: 'sys', tab: 'options' } });
    else if (top) segs.push({ text: top.short, to: routeOfIssue(h.issues[0].kind) });
    this.status.replaceChildren(...segs.map((s) => this.statusSegment(s.text, s.to)));
  }

  private statusSegment(text: string, to?: Route): HTMLElement {
    if (!to) return Object.assign(el('span'), { textContent: text });
    const b = Object.assign(el('button'), { textContent: text, type: 'button', tabIndex: -1 });
    b.addEventListener('click', (e) => {
      if (e.detail === 1) this.ctx.go(to);
    });
    return b;
  }

  /** 横带只显示一条；动作取那一行第一个不是「返回」的；首页上对齐后不自动开始 */
  private paintBanner(top: Copy | null): void {
    const found = top?.actions.find((a) => a !== 'back');
    const action = found === 'alignAndRetry' ? 'alignSuffix' : found;
    const tail = this.banner.show(top && {
      msg: top.text,
      action: action && { label: ACTION_TEXT[action], run: () => this.bannerAction(action) },
    });
    this.section.classList.toggle('iss', !!top);
    this.menu.setTail(tail);
    if (!tail && this.pinned === MENU_TEXT.length) this.pinned = -1;
  }

  /** 横带动作：被拒只提示、留在原处；重新指定后刷新首页 */
  private bannerAction(a: ActionId): void {
    const call = runAction(this.ctx, a)?.catch(toastError);
    if (a === 'chooseDir') void call?.finally(() => this.ctx.refreshHome());
  }

  /** 鼠标离开菜单（含横带动作）时，悬停选中的光标回到「开始游戏」（裁定 I）；单击过的项除外 */
  private onHover(t: Element): void {
    if (this.section.dataset.mode !== 'menu' || isDialogOpen()) return;
    const tail = this.banner.visible ? this.banner.el.querySelector('.opt') : null;
    const inside = this.menu.el.contains(t) || !!tail?.contains(t);
    if (inside) {
      // 悬停换到了别的项
      if (this.menu.selected !== this.pinned) this.pinned = -1;
    } else if (this.pointerIn && this.pinned < 0) {
      this.menu.select(0);
    }
    this.pointerIn = inside;
  }

  /** 单击（菜单自己的单击处理先跑，已选中并执行）：记下这一项 */
  private pinClicked(target: EventTarget | null, onBanner: boolean): void {
    if (!(target instanceof Element)) return;
    const i = onBanner
      ? (target.closest('.opt') ? MENU_TEXT.length : -1)
      : Number(target.closest<HTMLElement>('button[data-i]')?.dataset.i ?? -1);
    if (i >= 0 && i === this.menu.selected) this.pinned = i;
  }
}
