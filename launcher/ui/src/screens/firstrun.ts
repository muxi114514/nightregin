// 首次运行：首页画好后依次弹全宽横带。找游戏不能跳过，其余都能跳过；某一步调用失败提示后继续
import type { Issue } from '../api';
import { openDialog } from '../ui/dialog';
import { showToast, toastError } from '../ui/toast';
import type { AppCtx } from './app';
import { CHOOSE_DIR_FAIL_TEXT, issueCopy } from './texts';

const SUFFIX_KINDS = new Set<Issue['kind']>(['suffixMismatch', 'chanceSetterSuffix']);
const mask = (id: string) => `账号 …${id.slice(-4)}`;

async function findGame(ctx: AppCtx): Promise<void> {
  for (;;) {
    await openDialog({
      msg: '没有找到游戏',
      sub: '请选择含 Game\\nightreign.exe 的游戏根目录。',
      opts: ['选择文件夹'],
      noEsc: true,
    });
    const r = await ctx.api.chooseGameDir().catch((e: unknown) => {
      toastError(e);
      return null;
    });
    if (r?.kind === 'chosen') return;
    if (r?.kind === 'rejected') showToast(CHOOSE_DIR_FAIL_TEXT);
  }
}

async function fixIssues(ctx: AppCtx, issues: Issue[]): Promise<void> {
  if (issues.some((i) => SUFFIX_KINDS.has(i.kind))) await ctx.api.alignSuffix();
  if (issues.some((i) => i.kind === 'everdarkMismatch')) await ctx.api.fixEverdark();
}

export async function runFirstRun(ctx: AppCtx): Promise<void> {
  const { api } = ctx;
  const plan = await api.getFirstRun();
  if (plan.done) return;
  if (!plan.gameFound) await findGame(ctx);

  let account: string | undefined;
  if (plan.needAccountChoice) {
    const i = await openDialog({
      msg: '用哪个 Steam 账号的存档？',
      opts: [...plan.accounts.map(mask), '跳过'],
    });
    account = plan.accounts[i];
  }
  const target = account ?? plan.activeAccount ?? plan.accounts[0];
  if (plan.canCopyVanilla && target) {
    const i = await openDialog({
      msg: '从原版存档复制一份？',
      sub: '还没有整合包存档，但有原版存档。只复制，不动原版存档。',
      opts: ['复制一份', '跳过'],
    });
    if (i === 0) await api.copyFromVanilla(target).catch(toastError);
  }
  if (plan.issues.length) {
    const text = plan.issues.map((it) => issueCopy(it, false)?.text ?? it.kind).join('\n');
    const i = await openDialog({ msg: '整合包配置不一致', sub: text, opts: ['一键改正', '跳过'] });
    if (i === 0) await fixIssues(ctx, plan.issues).catch(toastError);
  }
  const i = await openDialog({ msg: '要在桌面建快捷方式吗？', sub: '启动器在游戏目录下三层，不好找。', opts: ['建立', '不用'] });
  if (i === 0) await api.createShortcut().catch(toastError);
  await api.finishFirstRun(account).catch(toastError);
  ctx.go({ page: 'home', row: 0 });
}
