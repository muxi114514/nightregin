// 和 Rust 的 serde 输出一一对应（core\tests\launch.rs 的 every_variant_json_shape、preflight.rs 的 home_state_json_shape 钉住）

export type Step = 'preflight' | 'backup' | 'loadConfig' | 'wake';

/** 声明顺序就是首页横带的优先级 */
export type IssueKind =
  | 'packNotFound' | 'gameNotFound' | 'gameVersionMismatch' | 'me3FilesMissing' | 'saveBusy'
  | 'suffixMismatch' | 'chanceSetterSuffix' | 'everdarkMismatch' | 'lastExitAbnormal' | 'steamNotRunning' | 'gameAlreadyRunning';

/** detail 为空时是 ""，不会省略 */
export interface Issue { kind: IssueKind; blocking: boolean; detail: string }

export interface DeepNight { enabled: boolean; level: number }

export interface HomeState {
  version: string | null;
  deepNight: DeepNight | null;
  issues: Issue[];
  lowEffects: boolean;
  devMode: boolean;
}

export type FailReason =
  | { kind: 'preflight'; issue: Issue }
  | { kind: 'saveBusy' }
  | { kind: 'accountUnknown'; accounts: string[] }
  | { kind: 'backup'; message: string }
  | { kind: 'configWrite'; message: string }
  | { kind: 'spawnFailed'; message: string }
  | { kind: 'me3Exit'; code: number }
  | { kind: 'gameNeverStarted' }
  | { kind: 'gameExitedEarly'; code: number | null };

export type LaunchView =
  | { phase: 'idle' }
  | { phase: 'step'; step: Step }
  | { phase: 'waking'; slow: boolean; gameLoading: boolean }
  | { phase: 'running' }
  | { phase: 'failed'; step: Step; reason: FailReason }
  | { phase: 'cancelled' }
  | { phase: 'finished'; abnormal: boolean };

export type ConfigFileId = 'nighter' | 'nrsc' | 'chanceSetter';
export type Field<T> = { state: 'ok'; value: T } | { state: 'failed'; file: ConfigFileId; message: string };

/** picked：选中必定、没选不会；factory：整合包默认几率；custom：手改的几率或永夜几率被关掉 */
export interface LordsView { list: number[]; odds: 'picked' | 'factory' | 'custom'; hasFactory: boolean }

export interface GameSettings {
  forceDeepNight: Field<boolean>;
  deepNightLevel: Field<number>;
  unlockDeepNight: Field<boolean>;
  lords: Field<LordsView>;
  lordPicks: number[] | null;
  healthScaling: Field<number>;
  damageScaling: Field<number>;
  postureScaling: Field<number>;
  gameRunning: boolean;
}

export type SettingChange =
  | { key: 'forceDeepNight' | 'unlockDeepNight'; value: boolean }
  | { key: 'deepNightLevel' | 'healthScaling' | 'damageScaling' | 'postureScaling'; value: number }
  | { key: 'lords'; value: number[] }
  | { key: 'lordsFactory' };

export interface SuffixView {
  me3: string | null; nrsc: string; chanceSetter: string | null;
  consistent: boolean; blocking: boolean; pathManual: boolean;
}

export interface BackupRow { name: string; kind: 'auto' | 'manual' | 'prerestore'; takenAt: string; size: number }

export interface SavesView {
  accounts: string[]; active: string | null; current: string | null; saveFile: string | null;
  backups: BackupRow[]; suffix: SuffixView; canCopyVanilla: boolean; gameRunning: boolean;
}

export type BackupNow =
  | { kind: 'needsConfirm' } | { kind: 'created'; row: BackupRow } | { kind: 'unchanged'; row: BackupRow } | { kind: 'noSave' };
export type VanillaCopy = 'copied' | 'alreadyHasSave' | 'noVanilla';

export type AfterLaunch = 'tray' | 'keep' | 'close';
export interface OptionsView {
  backupBeforeStart: boolean; afterLaunch: AfterLaunch; lowEffects: boolean; devMode: boolean; gameDir: string | null;
}
export type OptionChange =
  | { key: 'backupBeforeStart' | 'lowEffects' | 'devMode'; value: boolean }
  | { key: 'afterLaunch'; value: AfterLaunch };
export interface AboutView {
  packVersion: string | null; launcherVersion: string; me3Version: string | null;
  gameVersion: string | null; updateSource: string | null;
}

export interface DumpRow { name: string; size: number }
export interface ReportPreview { dumps: DumpRow[]; dumpBytes: number }

export interface FirstRunPlan {
  done: boolean; gameFound: boolean; accounts: string[]; activeAccount: string | null;
  needAccountChoice: boolean; canCopyVanilla: boolean; issues: Issue[];
}

export type ChooseDir = { kind: 'cancelled' } | { kind: 'rejected' } | { kind: 'chosen'; path: string };

export interface WikiStat { k: string; v: string; h?: boolean }
/** 带小标题的一段说明（武器的战技等） */
export interface WikiSection { t: string; v: string }
/** 除 id、name 外都可能没有；不认识的字段跳过 */
export interface WikiEntry {
  id: string; name: string; sub?: string; icon?: string; image?: string; portrait?: boolean; filters?: string[]; stats?: WikiStat[];
  desc?: string; lore?: string; sections?: WikiSection[]; source?: string; hand?: string[]; lord?: number; everIcon?: string; everName?: string;
  chance?: { table: string; key: string };
}
export interface WikiCategory {
  id: string; name: string; icon?: string; layout?: string; order?: number; sourceLabel?: string;
  filters?: string[]; emptyText?: string[]; entries?: WikiEntry[];
}
export interface WikiData { format: number; version?: string; categories: WikiCategory[] }
export type WikiView =
  | { state: 'ok'; data: WikiData; newIds: string[] } | { state: 'missing' }
  | { state: 'incompatible'; format: number } | { state: 'broken'; message: string };
export interface WikiOdds {
  level: number | null; nightlords: Record<string, number[]> | null;
  maps: Record<string, number> | null; everdark: Record<string, number[]> | null;
}

/** 前端对 Rust 的全部依赖；订阅函数在事件注册完成后 resolve 出取消订阅 */
export interface Api {
  uiReady(): Promise<void>;
  getHome(): Promise<HomeState>;
  /** 已在启动中时以「已在启动中」拒绝 */
  startGame(o: { skipBackup: boolean; ignoreVersion: boolean }): Promise<void>;
  /** 唤起之前才取消，返回 true */
  cancelLaunch(): Promise<boolean>;
  endProcesses(): Promise<void>;
  focusGame(): Promise<boolean>;
  /** 取消、选错、选中分开返回，选错要提示 */
  chooseGameDir(): Promise<ChooseDir>;
  getLaunchView(): Promise<LaunchView>;
  getGameSettings(): Promise<GameSettings>;
  /** 入队，停顿 300ms 后写回 */
  setGameSetting(change: SettingChange): Promise<void>;
  restoreConfigDefault(file: ConfigFileId): Promise<GameSettings>;
  fixEverdark(): Promise<void>;
  alignSuffix(): Promise<void>;
  getSaves(account: string | null): Promise<SavesView>;
  backupNow(account: string, force: boolean): Promise<BackupNow>;
  restoreBackup(account: string, name: string): Promise<BackupRow | null>;
  openSaveFolder(account: string): Promise<void>;
  setSaveAccount(account: string): Promise<void>;
  copyFromVanilla(account: string): Promise<VanillaCopy>;
  getOptions(): Promise<OptionsView>;
  setOption(change: OptionChange): Promise<void>;
  getAbout(): Promise<AboutView>;
  createShortcut(): Promise<string>;
  /** 告诉 Rust 界面收到了托盘的「整理问题记录」 */
  ackOpenReport(): Promise<void>;
  previewReport(): Promise<ReportPreview>;
  createReport(): Promise<string>;
  revealPath(path: string): Promise<void>;
  getFirstRun(): Promise<FirstRunPlan>;
  finishFirstRun(account?: string): Promise<void>;
  getWiki(): Promise<WikiView>;
  getWikiOdds(): Promise<WikiOdds>;
  /** rel 是 wiki.json 里的相对路径 */
  wikiImageUrl(rel: string): string;
  minimizeWindow(): Promise<void>;
  closeApp(): Promise<void>;
  onLaunchView(cb: (v: LaunchView) => void): Promise<() => void>;
  onHomeState(cb: (h: HomeState) => void): Promise<() => void>;
  onVisibility(cb: (visible: boolean) => void): Promise<() => void>;
  onCloseRequested(cb: () => void): Promise<() => void>;
  onGameSettings(cb: (v: GameSettings) => void): Promise<() => void>;
  onConfigWriteFailed(cb: (message: string) => void): Promise<() => void>;
  onOpenReport(cb: () => void): Promise<() => void>;
}
