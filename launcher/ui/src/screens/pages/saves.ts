// 存档：立即备份、打开文件夹、存档后缀、账号、从原版复制、备份列表（Enter 恢复）
import type { BackupRow, SavesView } from '../../api';
import { openDialog } from '../../ui/dialog';
import { asset } from '../../ui/dom';
import type { Row } from '../../ui/rows';
import { showToast, toastError } from '../../ui/toast';
import type { AppCtx } from '../app';
import type { SysHost, SysPage } from '../sys';

const KIND_TEXT: Record<BackupRow['kind'], string> = { auto: '自动', manual: '手动', prerestore: '恢复前' };
const NO_ACCOUNT = '未选择';
const NEED_ACCOUNT = '请先选择存档账号';
const FILES = ['褪色燃烧.me3 · savefile', 'SeamlessCoop\\nrsc_settings.ini · save_file_extension', 'dll\\ChanceSetter.ini · autoExtension'];

const mask = (id: string) => `账号 …${id.slice(-4)}`;
const plain = (ext: string | null | undefined) => (ext ?? '').replace(/^\./, '') || '（无）';
const megabytes = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`;

export function createSavesPage(ctx: AppCtx): SysPage {
  let view: SavesView | null = null;
  let account: string | null = null;
  let host: SysHost | null = null;
  /** 这次恢复前备份的文件名：离开存档页之前排在最上面 */
  let pinned: string | null = null;
  const { api } = ctx;

  async function load(): Promise<void> {
    view = await api.getSaves(account);
    account = view.current;
  }

  async function reload(row?: string): Promise<void> {
    try {
      await load();
      host?.reload(row);
    } catch (e) {
      toastError(e);
    }
  }

  function withAccount(run: (id: string) => void): void {
    if (account) run(account);
    else showToast(NEED_ACCOUNT);
  }

  async function backup(id: string, force: boolean): Promise<void> {
    try {
      const r = await api.backupNow(id, force);
      if (r.kind === 'needsConfirm') {
        const i = await openDialog({ msg: '游戏运行中，这次备份可能不完整', opts: ['仍然备份', '取消'], def: 1, esc: 1 });
        if (i === 0) await backup(id, true);
        return;
      }
      showToast(r.kind === 'created' ? '已备份' : r.kind === 'unchanged' ? '存档没有变化' : '还没有存档');
      await reload();
    } catch (e) {
      toastError(e);
    }
  }

  async function restore(row: BackupRow): Promise<void> {
    if (view?.gameRunning) {
      showToast('游戏运行中，不能恢复存档');
      return;
    }
    const i = await openDialog({
      msg: `恢复到 ${row.takenAt} 的备份？`,
      sub: '当前存档会先另存一份恢复前备份。',
      opts: ['恢复', '取消'],
      def: 1,
      esc: 1,
    });
    if (i !== 0 || !account) return;
    try {
      const pre = await api.restoreBackup(account, row.name);
      pinned = pre?.name ?? null;
      showToast('已恢复这份备份');
      await reload(pinned ?? row.name);
    } catch (e) {
      toastError(e);
    }
  }

  function backupRows(): Row[] {
    const list = [...(view?.backups ?? [])];
    if (!list.length) {
      return [{ id: 'none', label: '还没有备份', type: 'readonly', value: () => '', desc: () => ({ paragraphs: ['开始游戏前会自动备份，也可以按上面的「立即备份」。'] }) }];
    }
    list.sort((a, b) => Number(b.name === pinned) - Number(a.name === pinned));
    return list.map((b): Row => ({
      id: b.name,
      label: `${b.takenAt} · ${KIND_TEXT[b.kind]}`,
      type: 'action',
      value: () => megabytes(b.size),
      enter: () => '恢复',
      run: () => void restore(b),
      desc: () => ({
        img: asset('fl_copy.png'),
        meta: KIND_TEXT[b.kind],
        paragraphs: ['按 Enter 恢复到这份备份。恢复前会先把当前存档另存一份。'],
        data: [b.name],
      }),
    }));
  }

  function accountRow(): Row {
    const ids = view?.accounts ?? [];
    const note = () => (account && view?.active && account !== view.active
      ? ['Steam 当前登录的是另一个账号；开始游戏时的备份按登录账号。']
      : []);
    const desc = () => ({
      img: asset('tab_save.png'),
      paragraphs: ['存档按 Steam 账号分开放。这里切换的是存档页查看的账号。', ...note()],
      data: account ? [account] : [],
    });
    if (ids.length < 2) {
      return { id: 'account', label: '存档账号', type: 'readonly', value: () => (ids[0] ? mask(ids[0]) : NO_ACCOUNT), desc };
    }
    const options = account ? ids.map(mask) : [NO_ACCOUNT, ...ids.map(mask)];
    return {
      id: 'account', label: '存档账号', type: 'pick', options,
      get: () => ({ ok: true, value: account ? ids.indexOf(account) : 0 }),
      set: (i) => {
        const id = account ? ids[i] : ids[i - 1];
        if (!id) return;
        account = id;
        api.setSaveAccount(id).catch(toastError);
        void reload('account');
      },
      desc,
    };
  }

  const rows = (): Row[] => (!view ? [] : [
    {
      id: 'backup', label: '立即备份', type: 'action',
      run: () => withAccount((id) => void backup(id, false)),
      desc: () => ({
        img: asset('fl_copy.png'),
        paragraphs: ['把当前存档另存一份手动备份。手动备份最多留 3 份，不会被启动前的自动备份挤掉。'],
        lore: '倒在夜里的，可以从记下的地方重来。',
        data: view?.saveFile ? [view.saveFile] : [],
      }),
    },
    {
      id: 'folder', label: '打开存档文件夹', type: 'action',
      run: () => withAccount((id) => {
        api.openSaveFolder(id).catch(toastError);
      }),
      desc: () => ({ img: asset('tab_save.png'), paragraphs: ['在资源管理器里打开存档所在的文件夹。'] }),
    },
    {
      id: 'suffix', label: '存档后缀', type: 'action',
      value: () => (view?.suffix.consistent ? plain(view.suffix.me3) : '不一致'),
      enter: () => (view?.suffix.consistent ? '' : '统一后缀'),
      run: () => {
        if (view?.suffix.consistent) return;
        api.alignSuffix().then(() => reload()).catch(toastError);
      },
      desc: () => ({
        img: asset('fl_copy.png'),
        meta: view?.suffix.consistent ? '三处一致' : '三处不一致',
        paragraphs: [
          `启动配置 ${plain(view?.suffix.me3)}，无缝联机 ${plain(view?.suffix.nrsc)}，夜王几率 ${plain(view?.suffix.chanceSetter)}。`,
          ...(view?.suffix.pathManual ? ['ChanceSetter 的存档路径是手动指定的，要自己改。'] : []),
          ...(view?.suffix.consistent ? [] : ['按 Enter 把无缝联机和夜王几率对齐到启动配置的后缀。存档本身不动。']),
        ],
        dev: FILES,
      }),
    },
    accountRow(),
    ...(view.canCopyVanilla ? [{
      id: 'vanilla', label: '从原版存档复制', type: 'action' as const,
      run: () => withAccount((id) => {
        api.copyFromVanilla(id).then(() => {
          showToast('已复制一份');
          return reload();
        }).catch(toastError);
      }),
      desc: () => ({ img: asset('fl_copy.png'), paragraphs: ['还没有整合包存档，但有原版存档。只复制一份，不动原版存档。'] }),
    }] : []),
    { type: 'separator', id: 'sep' },
    ...backupRows(),
  ]);

  return {
    defaultRow: 'backup',
    rows,
    attach(h) {
      host = h;
    },
    async load() {
      pinned = null;
      await load();
    },
  };
}
