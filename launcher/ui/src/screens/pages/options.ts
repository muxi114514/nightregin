// 设置：启动器自己的选项、整理问题记录、游戏目录、桌面快捷方式、关于（连按 5 次切换开发模式）
import type { AboutView, AfterLaunch, OptionChange, OptionsView } from '../../api';
import { asset } from '../../ui/dom';
import type { Row } from '../../ui/rows';
import { showToast, toastError } from '../../ui/toast';
import type { AppCtx } from '../app';
import { runReport } from '../report';
import type { SysHost, SysPage } from '../sys';
import { CHOOSE_DIR_FAIL_TEXT } from '../texts';

const AFTER: AfterLaunch[] = ['tray', 'keep', 'close'];
const AFTER_TEXT = ['隐入托盘', '保持窗口', '关闭启动器'];
const DEV_PRESSES = 5;
const DEV_WINDOW_MS = 3000;
const UNKNOWN = '未知';

export function createOptionsPage(ctx: AppCtx): SysPage {
  let view: OptionsView | null = null;
  let about: AboutView | null = null;
  let host: SysHost | null = null;
  let presses: number[] = [];

  function send(change: OptionChange): void {
    ctx.api.setOption(change).catch(toastError);
  }

  function pressAbout(): void {
    const now = performance.now();
    presses = [...presses.filter((t) => now - t < DEV_WINDOW_MS), now];
    if (presses.length < DEV_PRESSES || !view) return;
    presses = [];
    view.devMode = !view.devMode;
    send({ key: 'devMode', value: view.devMode });
    showToast(view.devMode ? '已打开开发模式' : '已关闭开发模式');
  }

  async function chooseDir(): Promise<void> {
    try {
      const r = await ctx.api.chooseGameDir();
      if (r.kind === 'rejected') showToast(CHOOSE_DIR_FAIL_TEXT);
      else if (r.kind === 'chosen' && view) {
        view.gameDir = r.path;
        host?.refresh();
      }
    } catch (e) {
      toastError(e);
    }
  }

  function bool(id: 'backupBeforeStart' | 'lowEffects', label: string, img: string, text: string, lore?: string): Row {
    return {
      id, label, type: 'toggle',
      get: () => (view ? { ok: true, value: view[id] } : { ok: false, message: '' }),
      set: (v) => {
        if (!view) return;
        view[id] = v;
        send({ key: id, value: v });
      },
      desc: () => ({ img: asset(img), paragraphs: [text], lore }),
    };
  }

  const version = (v: string | null | undefined) => v ?? UNKNOWN;

  const rows = (): Row[] => (!view ? [] : [
    bool('backupBeforeStart', '启动前备份存档', 'fl_copy.png', '每次开始游戏前复制一份存档，保留最近 10 份；存档没有变化时不重复备份。手动备份另外保留，不参与轮换。', '记下此刻，免得长夜吞没来路。'),
    {
      id: 'afterLaunch', label: '游戏启动后', type: 'pick', options: AFTER_TEXT,
      get: () => ({ ok: true, value: AFTER.indexOf(view?.afterLaunch ?? 'tray') }),
      set: (i) => {
        if (!view) return;
        view.afterLaunch = AFTER[i];
        send({ key: 'afterLaunch', value: AFTER[i] });
      },
      desc: () => ({ img: asset('fl_sleep.png'), paragraphs: ['游戏窗口出现之后，启动器怎样处理自己。隐入托盘时动画全部停下，不和游戏抢资源。'] }),
    },
    bool('lowEffects', '低特效', 'fl_veil.png', '停掉启动器的余烬动画和柔光呼吸，只留静帧。不影响游戏画面。'),
    {
      id: 'report', label: '整理问题记录', type: 'action',
      run: () => void runReport(ctx),
      desc: () => ({ img: asset('fl_scroll.png'), paragraphs: ['把 me3 和各模组日志、配置文件、版本号和最近的崩溃转储打成一个 zip 放到桌面，方便发群求助。'] }),
    },
    {
      id: 'gameDir', label: '游戏所在文件夹', type: 'path', enter: '重新指定',
      value: () => view?.gameDir ?? '没有找到',
      run: () => void chooseDir(),
      desc: () => ({
        img: asset('fl_dresser.png'),
        paragraphs: ['第一次运行时自动找到。移动过游戏库时，按 Enter 重新指定。'],
        data: view?.gameDir ? [view.gameDir] : [],
      }),
    },
    {
      id: 'shortcut', label: '建桌面快捷方式', type: 'action',
      run: () => {
        ctx.api.createShortcut().then(() => showToast('已在桌面建好快捷方式')).catch(toastError);
      },
      desc: () => ({ img: asset('fl_toolbox.png'), paragraphs: ['启动器在游戏目录下三层，不好找，可以在桌面放一个快捷方式。'] }),
    },
    {
      id: 'about', label: '关于', type: 'action', rawClicks: true,
      value: () => version(about?.packVersion),
      enter: () => '',
      run: pressAbout,
      desc: () => ({
        img: asset('logo.webp'),
        paragraphs: ['黑夜君临黄金时代', 'Nightreign Golden Age by max-z'],
        data: [
          `整合包 ${version(about?.packVersion)}`,
          `启动器 ${version(about?.launcherVersion)}`,
          `me3 ${version(about?.me3Version)}`,
          `游戏 ${version(about?.gameVersion)}`,
          `更新源 ${about?.updateSource ?? '未配置'}`,
          '第三方许可见 launcher\\licenses',
        ],
      }),
    },
  ]);

  return {
    defaultRow: 'backupBeforeStart',
    rows,
    attach(h) {
      host = h;
    },
    async load() {
      [view, about] = await Promise.all([ctx.api.getOptions(), ctx.api.getAbout()]);
    },
  };
}
