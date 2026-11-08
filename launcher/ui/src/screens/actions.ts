// 横带动作 → 接口调用（任务书失败表的「动作」一栏），首页横带与失败横带共用；
// 被拒时怎么提示、之后回哪一屏由调用方决定
import { showToast } from '../ui/toast';
import type { AppCtx } from './app';
import { runReport } from './report';
import { CHOOSE_DIR_FAIL_TEXT, type ActionId } from './texts';

export interface StartJob { skipBackup: boolean; ignoreVersion: boolean }

/** 会重新开始游戏的动作；「开始游戏」与「重试」同参数 */
export const START_JOB: Record<'retry' | 'skipBackup' | 'ignoreVersion', StartJob> = {
  retry: { skipBackup: false, ignoreVersion: false },
  skipBackup: { skipBackup: true, ignoreVersion: false },
  ignoreVersion: { skipBackup: false, ignoreVersion: true },
};

export const isStartAction = (a: ActionId): a is keyof typeof START_JOB => Object.hasOwn(START_JOB, a);

/** 这次动作用的 job：连着重试时「仍然启动」不能丢 */
export function jobFor(a: keyof typeof START_JOB, previous?: StartJob): StartJob {
  const job = START_JOB[a];
  return { skipBackup: job.skipBackup, ignoreVersion: job.ignoreVersion || !!previous?.ignoreVersion };
}

/** 执行动作对应的接口调用；「返回」没有调用，为 null。job 是上一次开始游戏用的参数 */
export function runAction(ctx: AppCtx, a: ActionId, job?: StartJob): Promise<unknown> | null {
  const { api } = ctx;
  if (isStartAction(a)) return api.startGame(jobFor(a, job));
  switch (a) {
    case 'focusGame': return api.focusGame();
    case 'chooseDir':
      return api.chooseGameDir().then((r) => {
        if (r.kind === 'rejected') showToast(CHOOSE_DIR_FAIL_TEXT);
        return r;
      });
    case 'collectReport': return runReport(ctx);
    case 'alignSuffix': return api.alignSuffix();
    case 'alignAndRetry': return api.alignSuffix().then(() => api.startGame(job ?? START_JOB.retry));
    case 'fixEverdark': return api.fixEverdark();
    default: return null;
  }
}
