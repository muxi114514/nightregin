// 应用外壳：背景与余烬、拖动区、窗口按钮、动效暂停、Rust 事件订阅、页面切换；各页面只管自己的内容
import type { Api, HomeState, LaunchView } from '../api';
import { confirm } from '../ui/dialog';
import { asset, el } from '../ui/dom';
import { toastError } from '../ui/toast';
import { HomeScreen } from './home';
import { isLaunchBusy, runLaunch } from './launch';
import { createNightPage, type NightPage } from './pages/night';
import { createOptionsPage } from './pages/options';
import { createSavesPage } from './pages/saves';
import { runReport } from './report';
import { createSys, type Sys, type SysTab } from './sys';
import { createWiki, type Wiki } from './wiki';
import { CLOSE_TEXT, WINDOW_TEXT } from './texts';
import { showToast } from '../ui/toast';
import { inputGuard } from '../input/guard';

/** 从哪里跳进图鉴；Esc 回到那里，只记一层 */
export type WikiFrom = { page: 'lords'; focus: number } | { page: 'sys'; tab: SysTab; row: string };
export type Route =
  | { page: 'home'; row?: number }
  | { page: 'sys'; tab: SysTab; row?: string; lordFocus?: number }
  | { page: 'wiki'; id?: string; from?: WikiFrom };

export interface AppCtx {
  api: Api;
  devMode(): boolean;
  lowEffects(): boolean;
  go(to: Route): void;
  /** 重读首页状态，不算切页 */
  refreshHome(): void;
}

/** 这些进度要让玩家看到首页上的过场 */
const SHOWS_LAUNCH = new Set<LaunchView['phase']>(['step', 'waking', 'running', 'failed']);
const EMBER = { live: asset('ember.webp'), still: asset('ember_still.png') };

class App implements AppCtx {
  readonly api: Api;
  private readonly ember: HTMLImageElement;
  private readonly home: HomeScreen;
  private readonly sys: Sys;
  private readonly wiki: Wiki;
  private readonly night: NightPage;
  private readonly reduced = matchMedia('(prefers-reduced-motion: reduce)');
  private visible = true;
  private dev = false;
  private low = false;
  /** 每画一次首页加一；晚到的 getHome 结果不覆盖更新的状态 */
  private renderSeq = 0;
  private closing: AbortController | null = null;

  constructor(root: HTMLElement, api: Api) {
    this.api = api;
    const bg = el('div', 'bg');
    this.ember = el('img');
    this.ember.alt = '';
    this.ember.src = EMBER.live;
    bg.append(this.ember);
    const drag = el('div', 'drag');
    drag.setAttribute('data-tauri-drag-region', '');
    this.home = new HomeScreen(this);
    this.night = createNightPage(this);
    this.sys = createSys(this, { night: this.night, options: createOptionsPage(this), saves: createSavesPage(this) });
    this.wiki = createWiki(this);
    root.append(bg, el('div', 'dim'), drag, this.windowButtons(), this.home.section, this.sys.el, this.wiki.el);
    this.reduced.addEventListener('change', () => this.applyMotion());
    this.applyMotion();
    this.refresh();
  }

  devMode(): boolean {
    return this.dev;
  }

  lowEffects(): boolean {
    return this.low;
  }

  go(to: Route): void {
    inputGuard.markLayerChange(performance.now());
    if (to.page === 'wiki') {
      this.home.section.classList.remove('on');
      this.sys.close();
      this.wiki.open(to.id, to.from);
      return;
    }
    this.wiki.close();
    if (to.page === 'sys') {
      this.home.section.classList.remove('on');
      this.sys.open(to.tab, to.row);
      if (to.lordFocus !== undefined) this.night.openLords(to.lordFocus);
      return;
    }
    this.sys.close();
    this.home.section.classList.add('on');
    this.home.show(to.row ?? 0);
    this.refresh();
  }

  /** 全部事件都注册好才 resolve：之后才能报告「前端已就绪」 */
  async subscribe(): Promise<void> {
    const { api } = this;
    await Promise.all([
      api.onLaunchView((v) => this.onLaunchView(v)),
      api.onHomeState((h) => this.render(h)),
      api.onVisibility((visible) => {
        this.visible = visible;
        this.applyMotion();
      }),
      // 唤起前关窗口（Alt+F4 等）：Rust 拦下后交前端确认，同 ×（裁定 B）
      api.onCloseRequested(() => void this.requestClose(true)),
      api.onGameSettings((v) => this.night.apply(v)),
      api.onConfigWriteFailed((message) => showToast(`设置没有写入：${message}`)),
      api.onOpenReport(() => {
        api.ackOpenReport().catch(toastError);
        void runReport(this);
      }),
    ]);
    // 页面重载后接回启动过场
    const view = await api.getLaunchView().catch(() => null);
    if (view && (isLaunchBusy(view) || view.phase === 'running')) this.onLaunchView(view);
  }

  private windowButtons(): HTMLElement {
    const win = el('div', 'win');
    const button = (cls: string, title: string, bars: number, run: () => void) => {
      const b = el('button', cls);
      b.type = 'button';
      b.tabIndex = -1;
      b.title = title;
      b.setAttribute('aria-label', title);
      for (let i = 0; i < bars; i++) b.append(el('i'));
      b.addEventListener('click', run);
      return b;
    };
    win.append(
      button('', WINDOW_TEXT.minimize, 1, () => void this.api.minimizeWindow().catch(toastError)),
      button('x', WINDOW_TEXT.close, 2, () => void this.requestClose(false)),
    );
    return win;
  }

  private onLaunchView(v: LaunchView): void {
    if (v && !isLaunchBusy(v)) this.closing?.abort();
    // 过场画在首页上：从托盘开始游戏时可能正停在次级页
    if (v && SHOWS_LAUNCH.has(v.phase) && (this.sys.isOpen() || this.wiki.isOpen())) this.go({ page: 'home', row: 0 });
    runLaunch(this.home.section, v);
  }

  refreshHome(): void {
    this.refresh();
  }

  private refresh(): void {
    const seq = ++this.renderSeq;
    this.api.getHome()
      .then((h) => {
        if (seq === this.renderSeq) this.render(h);
      })
      .catch(toastError);
  }

  private render(h: HomeState): void {
    this.renderSeq++;
    if (!h || typeof h !== 'object') return;
    this.dev = h.devMode === true;
    this.low = h.lowEffects === true;
    this.applyMotion();
    this.home.render(h);
  }

  /** ×：启动进行中先确认（危险操作，默认「取消」），否则直接关；fromRust 为 close-requested，总是确认 */
  private async requestClose(fromRust: boolean): Promise<void> {
    if (this.closing) return;
    if (!fromRust && !this.home.launchInProgress()) {
      this.api.closeApp().catch(toastError);
      return;
    }
    const ctl = new AbortController();
    this.closing = ctl;
    const ok = await confirm({ ...CLOSE_TEXT, destructive: true }, ctl.signal);
    if (this.closing === ctl) this.closing = null;
    // Rust 自己会先取消唤起前的启动
    if (ok) this.api.closeApp().catch(toastError);
  }

  /** 窗口不可见、低特效或系统要求少动效：html 加 .paused，余烬换静帧 */
  private applyMotion(): void {
    const off = !this.visible || this.low || this.reduced.matches;
    document.documentElement.classList.toggle('paused', off);
    const want = off ? EMBER.still : EMBER.live;
    if (this.ember.getAttribute('src') !== want) this.ember.src = want;
  }
}

/** 在 root（#stage）里建应用；事件订阅注册完才 resolve */
export async function mountApp(root: HTMLElement, api: Api): Promise<AppCtx> {
  const app = new App(root, api);
  await app.subscribe();
  return app;
}
