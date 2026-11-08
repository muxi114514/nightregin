// 界面文案（裁定 D：screens 下的界面文字只在这里）。菜单、状态行、启动四步、唤起、失败表逐字取自任务书；
// 标「草图 v4」的任务书没有给，取自 reference\v4\index.html
import type { FailReason, Issue, IssueKind, Step } from '../api';

/** 草图 v4 */
export const LOGO_ALT = 'NIGHTREIGN GOLDEN AGE 黑夜君临黄金时代';
/** 草图 v4：窗口按钮的悬停提示 */
export const WINDOW_TEXT = { minimize: '最小化', close: '关闭' };

/** 首页菜单，顺序即显示顺序（没有「退出」） */
export const MENU_TEXT = ['开始游戏', '图鉴', '深夜与联机', '设置', '存档', '更新'];

export const STATUS_TEXT = {
  versionUnknown: '版本未知 · 请导入完整包',
  deepNight: (level: number) => `深夜 ${level} 层`,
  deepNightOff: '深夜关闭',
  ready: '一切就绪',
};

export const HINT_TEXT = {
  /** 首页可点的 Enter 提示（裁定 H） */
  ok: '确定',
  /** 以下草图 v4 */
  cancelLaunch: '取消启动',
  cancelNote: '「唤起游戏」开始后就不能取消',
  waking: '游戏正在唤起，已不能取消',
  /** 取消已受理、等 cancelled 时 */
  cancelling: '正在取消…',
};

export const STEP_TEXT: Record<Step, string> = { preflight: '预检', backup: '备份存档', loadConfig: '载入配置', wake: '唤起游戏' };

export const WAKE_TEXT = { loading: '游戏载入中', slow: '游戏仍在载入', endProcesses: '结束 me3 和游戏进程', keepWaiting: '继续等待' };

/** 游戏窗口出现后渐黑的画面上那一行 */
export const RUNNING_TEXT = '游戏运行中';

/** 收到不认识的失败原因（数据异常）时的提示 */
export const UNKNOWN_FAILURE_TEXT = '启动失败，原因未知';

/** 启动中关窗口的确认：问句与「关闭启动器」取自草图 v4，「取消」取自任务书 */
export const CLOSE_TEXT = { text: '启动还没有完成，要关闭启动器吗？', ok: '关闭启动器', cancel: '取消' };

/** 失败横带与首页横带的动作；调用见 launch.ts（START_JOB）与 home.ts */
export type ActionId = 'chooseDir' | 'ignoreVersion' | 'focusGame' | 'retry' | 'skipBackup' | 'back' | 'collectReport' | 'alignAndRetry' | 'alignSuffix' | 'fixEverdark';

/** 重新指定游戏文件夹被拒（选的位置里没有 Game\nightreign.exe）时的提示 */
export const CHOOSE_DIR_FAIL_TEXT = '这个文件夹里没有游戏，请选择含 Game\\nightreign.exe 的游戏根目录';

export const ACTION_TEXT: Record<ActionId, string> = {
  chooseDir: '重新指定游戏所在文件夹',
  ignoreVersion: '仍然启动',
  focusGame: '切到游戏',
  retry: '重试',
  skipBackup: '跳过这次备份',
  back: '返回',
  collectReport: '整理问题记录',
  alignAndRetry: '一键对齐后继续',
  alignSuffix: '一键改正',
  fixEverdark: '一键改正',
};

type RowKind = IssueKind | Exclude<FailReason['kind'], 'preflight'>;

/** devActions：开发模式下排在前面多出的动作；标「计划 2」的动作留空 */
interface Row { text: string; actions: ActionId[]; devActions?: ActionId[]; def?: number }

/** 任务书的失败表（saveBusy 首页问题与备份失败共用一行） */
const ROWS: Record<RowKind, Row> = {
  packNotFound: { text: '找不到整合包：请把启动器放在整合包目录里运行', actions: ['back'] },
  gameNotFound: { text: '找不到游戏：整合包上级目录和 Steam 库里都没有 nightreign.exe', actions: ['chooseDir', 'collectReport', 'back'] },
  gameVersionMismatch: { text: '游戏版本和整合包不符（{detail}），游戏更新后要等整合包更新', actions: ['back'], devActions: ['ignoreVersion'] },
  me3FilesMissing: { text: '缺少 me3 或模组文件：{detail}，请重新解压整合包', actions: ['collectReport', 'back'], def: 1 },
  steamNotRunning: { text: 'Steam 没有运行，请先打开 Steam 并登录', actions: ['retry', 'back'] },
  gameAlreadyRunning: { text: '游戏已经在运行', actions: ['focusGame', 'back'] },
  suffixMismatch: { text: '存档后缀不一致：{detail}，两边会读写不同的存档', actions: ['alignAndRetry', 'back'] },
  saveBusy: { text: '存档正被其他程序占用，备份失败', actions: ['retry', 'skipBackup', 'back'] },
  accountUnknown: { text: '无法确定用哪个 Steam 账号的存档（找到 {n} 个），请登录 Steam 后重试', actions: ['retry', 'skipBackup', 'back'] },
  backup: { text: '备份失败：{message}', actions: ['retry', 'skipBackup', 'back'] },
  configWrite: { text: '配置写入失败：{message}', actions: ['retry', 'collectReport', 'back'] },
  spawnFailed: { text: '无法运行 me3：{message}', actions: ['collectReport', 'retry', 'back'], def: 1 },
  me3Exit: { text: 'me3 启动失败（退出码 {code}）', actions: ['collectReport', 'retry', 'back'], def: 1 },
  gameNeverStarted: { text: 'me3 已退出，但游戏没有启动', actions: ['collectReport', 'retry', 'back'], def: 1 },
  gameExitedEarly: { text: '游戏在出现窗口前就退出了', actions: ['collectReport', 'retry', 'back'], def: 1 },
  chanceSetterSuffix: { text: '配置不一致：ChanceSetter 的存档后缀是 {detail}，夜王防重复和地图记录不会生效', actions: ['alignSuffix'] },
  everdarkMismatch: { text: '配置不一致：永夜王由夜王几率模组决定，但 nighter 的永夜名单被改过', actions: ['fixEverdark'] },
  lastExitAbnormal: { text: '上次游戏异常退出', actions: ['collectReport'] },
};

/** 一条横带：完整文案、简短名（状态行就绪段、失败那一步）、动作、默认（Enter）选第几个动作 */
export interface Copy { text: string; short: string; actions: ActionId[]; def: number }

const str = (v: unknown) => (typeof v === 'string' ? v : '');

function copyOf(kind: string, vars: Record<string, string>, devMode: boolean): Copy | null {
  if (!Object.hasOwn(ROWS, kind)) return null;
  const row = ROWS[kind as RowKind];
  const dev = devMode ? row.devActions ?? [] : [];
  return {
    text: row.text.replace(/\{(\w+)\}/g, (_, k: string) => vars[k] ?? ''),
    // 简短名 = 文案开头到第一个「：（，」之前，逐字取自表格
    short: /^[^：（，]+/.exec(row.text)?.[0] ?? row.text,
    actions: [...dev, ...row.actions],
    // 开发模式多出的动作（如「仍然启动」）排在前面但不作默认，Enter 不会误按到
    def: dev.length + (row.def ?? 0),
  };
}

/** 首页横带：issue 那一行，{detail} = issue.detail；不认识的问题为 null */
export function issueCopy(issue: Issue, devMode: boolean): Copy | null {
  if (!issue || typeof issue !== 'object') return null;
  // 这一行只有首页会用；失败横带的「存档被占用」是 FailReason，走 failureCopy
  if (issue.kind === 'saveBusy') {
    return { text: '存档正被其他程序占用，关掉占用它的程序后再开始游戏', short: '存档正被其他程序占用', actions: [], def: 0 };
  }
  return copyOf(issue.kind, { detail: str(issue.detail) }, devMode);
}

/** 失败横带：preflight 按 issue.kind 那一行（裁定 C），其余按自身那一行；不认识的原因为 null */
export function failureCopy(reason: FailReason, devMode: boolean): Copy | null {
  if (!reason || typeof reason !== 'object') return null;
  switch (reason.kind) {
    case 'preflight': return issueCopy(reason.issue, devMode);
    case 'accountUnknown': return copyOf(reason.kind, { n: String(Array.isArray(reason.accounts) ? reason.accounts.length : 0) }, devMode);
    case 'me3Exit': return copyOf(reason.kind, { code: typeof reason.code === 'number' ? String(reason.code) : '' }, devMode);
    case 'backup':
    case 'configWrite':
    case 'spawnFailed': return copyOf(reason.kind, { message: str(reason.message) }, devMode);
    // gameExitedEarly 的退出码不进文案
    default: return copyOf(String((reason as { kind: unknown }).kind), {}, devMode);
  }
}
