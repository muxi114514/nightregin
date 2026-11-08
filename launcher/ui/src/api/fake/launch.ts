// 假接口（URL 带 ?fake=<剧本名>，探测用）：照 Rust 的行为按固定间隔推 LaunchView，调用依次记进 window.__calls。
// 剧本：home-ok / home-nogame / home-version（首页状态；开始游戏时有阻断问题就在预检失败，否则正常启动）、
// launch-ok、launch-slow、launch-fail-<kind>（kind 取 FailReason.kind 或 IssueKind；saveBusy 两边都有，按 FailReason 即备份失败）。
// URL 加 &nodir=1 时 chooseGameDir 返回 null（选的文件夹里没有游戏）。
// 设置相关的状态见 settings.ts。
// 跳过备份、开发模式（URL 加 &dev=1）下仍然启动、重新指定游戏目录之后，对应的失败不再出现，同 Rust。
// launch-slow：四步后先「仍在载入、还没有游戏进程」，隔几拍游戏进程出现（slow 不变）；launch-hold：停在备份那一步，等 __fakeEmit。
// window.__fakeEmit(事件名, 负载) 补发浏览器里造不出来的事件（close-requested、window-visibility、finished 等）；
// 推过的 LaunchView 依次记进 window.__views
import type { Api, FailReason, HomeState, Issue, IssueKind, LaunchView, Step } from '../types';
import { createFakeSettings } from './settings';
import { createFakeWiki } from './wiki';

/** 相邻两个 LaunchView 的间隔 */
export const FAKE_STEP_MS = 500;
export const FAKE_GAME_DIR = 'D:\\Steam Library\\steamapps\\common\\ELDEN RING NIGHTREIGN';

export interface FakeCall { name: string; args?: unknown }

declare global {
  interface Window {
    __calls?: FakeCall[];
    __views?: LaunchView[];
    __fakeEmit?: (event: string, payload?: unknown) => void;
  }
}

type ReasonKind = FailReason['kind'];
type Plan = { kind: 'ok' } | { kind: 'slow' } | { kind: 'hold' } | { kind: 'fail'; reason: FailReason } | { kind: 'chain' };
interface Job { skipBackup: boolean; ignoreVersion: boolean }

const STEPS: Step[] = ['preflight', 'backup', 'loadConfig', 'wake'];

/** detail 的样例，格式同 core 的 preflight.rs */
const ISSUE_DETAIL: Record<IssueKind, string> = {
  packNotFound: '',
  gameNotFound: '',
  gameVersionMismatch: '实际 1.3.4.0 / 118835320，需要 1.3.3.0 / 118835320',
  me3FilesMissing: 'bin\\me3.exe、bin\\me3_mod_host.dll',
  saveBusy: 'NR0000.co2',
  suffixMismatch: '褪色燃烧.me3 是 co2，无缝联机是 sl2',
  chanceSetterSuffix: '.CL_SAVE',
  everdarkMismatch: '',
  lastExitAbnormal: '',
  steamNotRunning: '',
  gameAlreadyRunning: 'pid 4242',
};

const MESSAGE = '拒绝访问。 (os error 5)';
const REASONS: Record<Exclude<ReasonKind, 'preflight'>, FailReason> = {
  saveBusy: { kind: 'saveBusy' },
  accountUnknown: { kind: 'accountUnknown', accounts: ['76561198000000001', '76561198000000002'] },
  backup: { kind: 'backup', message: MESSAGE },
  configWrite: { kind: 'configWrite', message: MESSAGE },
  spawnFailed: { kind: 'spawnFailed', message: '系统找不到指定的文件。 (os error 2)' },
  me3Exit: { kind: 'me3Exit', code: 1 },
  gameNeverStarted: { kind: 'gameNeverStarted' },
  gameExitedEarly: { kind: 'gameExitedEarly', code: 1 },
};

/** 各原因在哪一步失败，同 core 的 launch.rs */
const FAIL_STEP: Record<ReasonKind, Step> = {
  preflight: 'preflight', saveBusy: 'backup', accountUnknown: 'backup', backup: 'backup',
  configWrite: 'loadConfig', spawnFailed: 'wake', me3Exit: 'wake', gameNeverStarted: 'wake', gameExitedEarly: 'wake',
};

const issue = (kind: IssueKind): Issue => ({ kind, blocking: kind !== 'chanceSetterSuffix' && kind !== 'everdarkMismatch' && kind !== 'lastExitAbnormal', detail: ISSUE_DETAIL[kind] });
const step = (s: Step): LaunchView => ({ phase: 'step', step: s });
const waking = (slow: boolean, gameLoading = true): LaunchView => ({ phase: 'waking', slow, gameLoading });
/** launch-slow 里游戏进程晚几拍出现，留时间给探测先选「继续等待」 */
const SLOW_GAME_LAG = 4;
/** &slowlisten=1 时订阅晚多久生效 */
const SLOW_LISTEN_MS = 200;
const isTerminal = (v: LaunchView) => v.phase === 'failed' || v.phase === 'cancelled' || v.phase === 'finished';
const beforeWake = (v: LaunchView) => v.phase === 'idle' || (v.phase === 'step' && v.step !== 'wake');

function failReason(kind: string): FailReason | null {
  if (Object.hasOwn(REASONS, kind)) return REASONS[kind as keyof typeof REASONS];
  if (kind === 'preflight') return { kind: 'preflight', issue: issue('gameNotFound') };
  if (Object.hasOwn(ISSUE_DETAIL, kind)) return { kind: 'preflight', issue: issue(kind as IssueKind) };
  return null;
}

/** plan 为 null：开始游戏时按首页的阻断问题决定 */
function parseScript(name: string): { issues: Issue[]; plan: Plan | null } | null {
  switch (name) {
    case 'home-ok': return { issues: [], plan: null };
    case 'home-nogame': return { issues: [issue('gameNotFound')], plan: null };
    case 'home-version': return { issues: [issue('gameVersionMismatch')], plan: null };
    case 'launch-ok': return { issues: [], plan: { kind: 'ok' } };
    case 'launch-slow': return { issues: [], plan: { kind: 'slow' } };
    case 'launch-chain': return { issues: [], plan: { kind: 'chain' } };
    case 'launch-hold':
    case 'launch-early': return { issues: [], plan: { kind: 'hold' } };
  }
  const m = /^launch-fail-(\w+)$/.exec(name);
  const reason = m ? failReason(m[1]) : null;
  return reason ? { issues: [], plan: { kind: 'fail', reason } } : null;
}

export function createFakeApi(name: string, o: { devMode?: boolean; noGameDir?: boolean; slowListen?: boolean; q?: URLSearchParams } = {}): Api {
  const script = parseScript(name);
  const q = o.q ?? new URLSearchParams();
  const home: HomeState = {
    version: null, deepNight: { enabled: true, level: 5 }, issues: script?.issues ?? [],
    lowEffects: false, devMode: !!o.devMode,
  };
  const calls: FakeCall[] = [];
  window.__calls = calls;
  const seen: LaunchView[] = [];
  window.__views = seen;
  const bus = new Map<string, Set<(payload: unknown) => void>>();
  let timers: number[] = [];
  let current: LaunchView = { phase: 'idle' };
  let active = false;
  let hidden = false;
  let gameDirChosen = false;

  if (!script) console.error(`未知的假剧本：${name}`);
  const unknown = () => Promise.reject(new Error(`未知的假剧本：${name}`));
  const record = (call: string, args?: unknown) => calls.push(args === undefined ? { name: call } : { name: call, args });

  function emit(event: string, payload?: unknown) {
    for (const cb of bus.get(event) ?? []) {
      try {
        cb(payload);
      } catch (err) {
        console.error(`处理假事件 ${event} 出错`, err);
      }
    }
  }
  async function on<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
    const set = bus.get(event) ?? new Set();
    bus.set(event, set);
    const fn = cb as (payload: unknown) => void;
    // &slowlisten=1：订阅晚一点才生效，同 Tauri 的异步 listen
    if (o.slowListen) await new Promise((r) => window.setTimeout(r, SLOW_LISTEN_MS));
    set.add(fn);
    return () => set.delete(fn);
  }
  function clearTimers() {
    timers.forEach(clearTimeout);
    timers = [];
  }
  /** 窗口的变化同 Rust：游戏窗口出现后隐入托盘；结束时显示回来，finished 另推一次首页状态 */
  function push(v: LaunchView) {
    current = v;
    seen.push(structuredClone(v));
    emit('launch-view', structuredClone(v));
    if (v.phase === 'running') {
      hidden = true;
      emit('window-visibility', { visible: false });
    }
    if (!isTerminal(v)) return;
    active = false;
    clearTimers();
    if (hidden) {
      hidden = false;
      emit('window-visibility', { visible: true });
    }
    if (v.phase === 'finished') emit('home-state', structuredClone(home));
  }
  /** 取消、结束进程都隔一拍才发 cancelled（同 Rust：启动线程下一次检查才停），界面看得到「正在取消…」 */
  function cancelSoon() {
    clearTimers();
    timers.push(window.setTimeout(() => push({ phase: 'cancelled' }), FAKE_STEP_MS));
  }

  /** 这次会被跳过的失败：跳过备份、开发模式仍然启动、已重新指定游戏目录 */
  function skipped(r: FailReason, job: Job): boolean {
    if (FAIL_STEP[r.kind] === 'backup') return job.skipBackup;
    if (r.kind !== 'preflight') return false;
    if (r.issue.kind === 'gameVersionMismatch') return job.ignoreVersion && home.devMode;
    return r.issue.kind === 'gameNotFound' && gameDirChosen;
  }
  function views(job: Job): LaunchView[] {
    let plan = script?.plan ?? null;
    if (!plan) {
      const first = home.issues.find((i) => i.blocking);
      plan = first ? { kind: 'fail', reason: { kind: 'preflight', issue: first } } : { kind: 'ok' };
    }
    const ok = [...STEPS.map(step), waking(false), { phase: 'running' } as LaunchView];
    if (plan.kind === 'slow') return [...STEPS.map(step), waking(true, false), waking(true, true)];
    if (plan.kind === 'hold') return [step('preflight'), step('backup')];
    if (plan.kind === 'chain') {
      const r: FailReason = job.ignoreVersion ? REASONS.saveBusy : { kind: 'preflight', issue: issue('gameVersionMismatch') };
      const at = FAIL_STEP[r.kind];
      return [...STEPS.slice(0, STEPS.indexOf(at) + 1).map(step), { phase: 'failed', step: at, reason: r }];
    }
    if (plan.kind === 'ok' || skipped(plan.reason, job)) return ok;
    const at = FAIL_STEP[plan.reason.kind];
    const upto = STEPS.slice(0, STEPS.indexOf(at) + 1).map(step);
    const loading = plan.reason.kind === 'gameExitedEarly' ? [waking(false)] : [];
    return [...upto, ...loading, { phase: 'failed', step: at, reason: plan.reason }];
  }

  window.__fakeEmit = (event, payload) => {
    if (event === 'launch-view') push(payload as LaunchView);
    else emit(event, payload);
  };

  return {
    uiReady: async () => {
      record('uiReady');
      // 托盘极早点了开始游戏：前端刚报告就绪，进度就到了
      if (name === 'launch-early') push(step('preflight'));
    },
    getHome: async () => {
      record('getHome');
      return script ? structuredClone(home) : unknown();
    },
    startGame: async (job) => {
      record('startGame', { ...job });
      if (!script) return unknown();
      if (active) throw new Error('已在启动中');
      active = true;
      current = { phase: 'idle' };
      const list = views(job);
      const lag = script?.plan?.kind === 'slow' ? SLOW_GAME_LAG : 0;
      list.forEach((v, i) => {
        const at = i === list.length - 1 ? i + lag : i;
        timers.push(window.setTimeout(() => push(v), at * FAKE_STEP_MS));
      });
    },
    cancelLaunch: async () => {
      record('cancelLaunch');
      const ok = active && beforeWake(current);
      if (ok) cancelSoon();
      return ok;
    },
    endProcesses: async () => {
      record('endProcesses');
      if (active) cancelSoon();
    },
    focusGame: async () => {
      record('focusGame');
      return true;
    },
    getLaunchView: async () => {
      record('getLaunchView');
      return q.get('view') === 'waking' ? waking(false) : current;
    },
    minimizeWindow: async () => { record('minimizeWindow'); },
    closeApp: async () => { record('closeApp'); },
    onLaunchView: (cb) => on('launch-view', cb),
    onHomeState: (cb) => on('home-state', cb),
    onVisibility: (cb) => on<{ visible?: unknown }>('window-visibility', (p) => {
      if (typeof p?.visible === 'boolean') cb(p.visible);
    }),
    onCloseRequested: (cb) => on('close-requested', () => cb()),
    onGameSettings: (cb) => on('game-settings', cb),
    onConfigWriteFailed: (cb) => on('config-write-failed', cb),
    onOpenReport: (cb) => on('open-report', () => cb()),
    ...createFakeWiki(q, record),
    ...createFakeSettings({
      q, home, record, emit, noGameDir: !!o.noGameDir,
      onChosen: () => {
        gameDirChosen = true;
        home.issues = home.issues.filter((i) => i.kind !== 'gameNotFound');
      },
    }),
  };
}
