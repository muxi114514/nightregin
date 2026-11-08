// 整理问题记录：有崩溃转储时先列给玩家看（转储没法脱敏），再生成；设置页、横带、托盘共用
import { openDialog } from '../ui/dialog';
import { showToast, toastError } from '../ui/toast';
import type { AppCtx } from './app';

let busy = false;

const mb = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`;

export async function runReport(ctx: AppCtx): Promise<void> {
  if (busy) return;
  busy = true;
  try {
    const preview = await ctx.api.previewReport();
    if (preview.dumps.length) {
      const list = preview.dumps.map((d) => `${d.name}（${mb(d.size)}）`).join('、');
      const i = await openDialog({
        msg: '要带上最近的崩溃转储吗？',
        sub: '崩溃转储可能包含账号名等信息。',
        dt: `${list}，共 ${mb(preview.dumpBytes)}`,
        opts: ['生成', '取消'],
        def: 1,
        esc: 1,
      });
      if (i !== 0) return;
    }
    showToast('正在整理…');
    const path = await ctx.api.createReport();
    showToast('问题记录已放到桌面');
    ctx.api.revealPath(path).catch(toastError);
  } catch (e) {
    toastError(e);
  } finally {
    busy = false;
  }
}
