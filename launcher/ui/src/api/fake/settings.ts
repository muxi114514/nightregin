// 假接口的设置部分（探测用）：URL 参数决定状态
// &cfg=handEdited|nighterBad|csBad|picks|all|custom|running、&saves=two|vanilla|mismatch|running|empty、
// &dumps=1、&reportslow=1、&reportfail=1、&dircancel=1、&first=full|missing|copyfail、&view=waking
import type {
  AboutView, Api, BackupRow, FirstRunPlan, GameSettings, HomeState, OptionsView, SavesView, SuffixView,
} from '../types';
import { FAKE_GAME_DIR } from './launch';

export interface FakeCtx {
  q: URLSearchParams;
  home: HomeState;
  record(name: string, args?: unknown): void;
  emit(event: string, payload?: unknown): void;
  noGameDir: boolean;
  onChosen(): void;
}

type SettingsApi = Pick<Api,
  'getGameSettings' | 'setGameSetting' | 'restoreConfigDefault' | 'fixEverdark' | 'alignSuffix'
  | 'getSaves' | 'backupNow' | 'restoreBackup' | 'openSaveFolder' | 'setSaveAccount' | 'copyFromVanilla'
  | 'getOptions' | 'setOption' | 'getAbout' | 'createShortcut'
  | 'ackOpenReport' | 'previewReport' | 'createReport' | 'revealPath' | 'getFirstRun' | 'finishFirstRun' | 'chooseGameDir'>;

const A = '76561198000000001';
const B = '76561198000000002';
const ok = <T>(value: T) => ({ state: 'ok' as const, value });
const bad = { state: 'failed' as const, file: 'nighter' as const, message: '格式错误：第 3 行' };
const MB = 1024 * 1024;
const picked = (list: number[]) => ok({ list, odds: 'picked' as const, hasFactory: true });
const factoryOdds = ok({ list: [] as number[], odds: 'factory' as const, hasFactory: true });

declare global {
  interface Window { __fakeGameSettings?: (o: Partial<GameSettings>) => GameSettings }
}

function gameSettings(cfg: string | null): GameSettings {
  const s: GameSettings = {
    forceDeepNight: ok(true), deepNightLevel: ok(5), unlockDeepNight: ok(true),
    lords: factoryOdds, lordPicks: null,
    healthScaling: ok(120), damageScaling: ok(60), postureScaling: ok(100), gameRunning: false,
  };
  if (cfg === 'handEdited') {
    s.healthScaling = ok(125);
    s.damageScaling = ok(500);
  } else if (cfg === 'nighterBad') {
    s.forceDeepNight = bad;
    s.deepNightLevel = bad;
    s.unlockDeepNight = bad;
  } else if (cfg === 'csBad') {
    s.lords = { state: 'failed', file: 'chanceSetter', message: '缺少 [everdarkChances] enabled' };
  } else if (cfg === 'picks') {
    s.lords = picked([1, 3]);
    s.lordPicks = [1, 3];
  } else if (cfg === 'all') {
    s.lords = picked([0, 1, 2, 3, 4, 5, 6, 7]);
  } else if (cfg === 'custom') {
    s.lords = ok({ list: [], odds: 'custom', hasFactory: true });
  } else if (cfg === 'running') {
    s.gameRunning = true;
  }
  return s;
}

const backup = (at: string, kind: BackupRow['kind'], stamp: string): BackupRow => ({
  name: `NR0000_${stamp}_${kind}.co2.bak`, kind, takenAt: at, size: 20 * MB,
});

function savesView(mode: string | null): SavesView {
  const suffix: SuffixView = { me3: 'NR0000.co2'.slice(-3), nrsc: 'co2', chanceSetter: '.co2', consistent: true, blocking: false, pathManual: false };
  const v: SavesView = {
    accounts: [A], active: A, current: A,
    saveFile: `C:\\Users\\player\\AppData\\Roaming\\Nightreign\\${A}\\NR0000.co2`,
    backups: [backup('2026-11-06 21:40:05', 'auto', '2026-11-06_214005'), backup('2026-11-05 20:10:00', 'manual', '2026-11-05_201000')],
    suffix, canCopyVanilla: false, gameRunning: false,
  };
  if (mode === 'two') v.accounts = [A, B];
  if (mode === 'vanilla') {
    v.canCopyVanilla = true;
    v.saveFile = null;
  }
  if (mode === 'mismatch') v.suffix = { ...suffix, nrsc: 'sl2', consistent: false, blocking: true };
  if (mode === 'running') v.gameRunning = true;
  if (mode === 'empty') v.backups = [];
  return v;
}

function firstRun(mode: string | null): FirstRunPlan {
  const base: FirstRunPlan = {
    done: true, gameFound: true, accounts: [A], activeAccount: A, needAccountChoice: false, canCopyVanilla: false, issues: [],
  };
  if (mode === 'full') {
    return {
      ...base, done: false, accounts: [A, B], activeAccount: null, needAccountChoice: true, canCopyVanilla: true,
      issues: [{ kind: 'suffixMismatch', blocking: true, detail: 'x' }],
    };
  }
  if (mode === 'missing') return { ...base, done: false, gameFound: false };
  if (mode === 'copyfail') return { ...base, done: false, canCopyVanilla: true };
  return base;
}

export function createFakeSettings(c: FakeCtx): SettingsApi {
  const { q, record, emit, home } = c;
  let game = gameSettings(q.get('cfg'));
  window.__fakeGameSettings = (o) => ({ ...gameSettings(q.get('cfg')), ...o });
  const savesMode = q.get('saves');
  let saves = savesView(savesMode);
  const options: OptionsView = { backupBeforeStart: true, afterLaunch: 'tray', lowEffects: false, devMode: home.devMode, gameDir: FAKE_GAME_DIR };
  const about: AboutView = { packVersion: '1.9.5', launcherVersion: '0.1.0', me3Version: '0.10.1', gameVersion: '1.3.3.0', updateSource: null };
  const emitHome = () => emit('home-state', structuredClone(home));
  const dropIssue = (...kinds: string[]) => {
    home.issues = home.issues.filter((i) => !kinds.includes(i.kind));
    emitHome();
  };

  return {
    getGameSettings: async () => {
      record('getGameSettings');
      return structuredClone(game);
    },
    setGameSetting: async (change) => {
      record('setGameSetting', { change });
      if (change.key === 'lords') game.lords = picked(change.value);
      else if (change.key === 'lordsFactory') game.lords = factoryOdds;
      else game = { ...game, [change.key]: ok(change.value) } as GameSettings;
    },
    restoreConfigDefault: async (file) => {
      record('restoreConfigDefault', { file });
      game = gameSettings(null);
      return structuredClone(game);
    },
    fixEverdark: async () => {
      record('fixEverdark');
      dropIssue('everdarkMismatch');
    },
    alignSuffix: async () => {
      record('alignSuffix');
      saves.suffix = { ...saves.suffix, nrsc: 'co2', chanceSetter: '.co2', consistent: true, blocking: false };
      dropIssue('suffixMismatch', 'chanceSetterSuffix');
    },
    getSaves: async (account) => {
      record('getSaves', { account });
      if (account) saves.current = account;
      return structuredClone(saves);
    },
    backupNow: async (account, force) => {
      record('backupNow', { account, force });
      if (saves.gameRunning && !force) return { kind: 'needsConfirm' };
      if (savesMode === 'unchanged') return { kind: 'unchanged', row: saves.backups[0] };
      const row = backup('2026-11-08 12:00:00', 'manual', '2026-11-08_120000');
      saves.backups = [row, ...saves.backups];
      return { kind: 'created', row };
    },
    restoreBackup: async (account, name) => {
      record('restoreBackup', { account, name });
      if (saves.gameRunning) throw new Error('游戏运行中，不能恢复存档');
      const row = backup('2026-11-08 12:01:00', 'prerestore', '2026-11-08_120100');
      saves.backups = [...saves.backups, row];
      return row;
    },
    openSaveFolder: async (account) => { record('openSaveFolder', { account }); },
    setSaveAccount: async (account) => { record('setSaveAccount', { account }); },
    copyFromVanilla: async (account) => {
      record('copyFromVanilla', { account });
      if (q.get('first') === 'copyfail') throw new Error('复制失败：拒绝访问');
      saves = { ...saves, canCopyVanilla: false };
      return 'copied';
    },
    getOptions: async () => {
      record('getOptions');
      return { ...options };
    },
    setOption: async (change) => {
      record('setOption', { change });
      Object.assign(options, { [change.key]: change.value });
      if (change.key === 'lowEffects') home.lowEffects = change.value;
      if (change.key === 'devMode') home.devMode = change.value;
      emitHome();
    },
    getAbout: async () => {
      record('getAbout');
      return { ...about };
    },
    createShortcut: async () => {
      record('createShortcut');
      return 'C:\\Users\\player\\Desktop\\黑夜君临黄金时代.lnk';
    },
    ackOpenReport: async () => { record('ackOpenReport'); },
    previewReport: async () => {
      record('previewReport');
      return q.get('dumps') === '1'
        ? { dumps: [{ name: 'a.dmp', size: 9 * MB }, { name: 'b.dmp', size: 9 * MB }], dumpBytes: 18 * MB }
        : { dumps: [], dumpBytes: 0 };
    },
    createReport: async () => {
      record('createReport');
      if (q.get('reportfail') === '1') throw new Error('写问题记录失败：磁盘已满');
      if (q.get('reportslow') === '1') await new Promise((r) => window.setTimeout(r, 1000));
      return 'C:\\Users\\player\\Desktop\\黄金时代问题记录_2026-11-08_1200.zip';
    },
    revealPath: async (path) => { record('revealPath', { path }); },
    getFirstRun: async () => {
      record('getFirstRun');
      return firstRun(q.get('first'));
    },
    finishFirstRun: async (account) => {
      record('finishFirstRun', account ? { account } : {});
    },
    chooseGameDir: async () => {
      record('chooseGameDir');
      if (q.get('dircancel') === '1') return { kind: 'cancelled' };
      if (c.noGameDir) return { kind: 'rejected' };
      c.onChosen();
      return { kind: 'chosen', path: FAKE_GAME_DIR };
    },
  };
}
