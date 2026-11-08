// 入口：装键鼠规则、建 1200×720 舞台并等比缩放、挂首页；首屏画完再通知 Rust 显示窗口
import './styles/base.css';
import './styles/dialog.css';
import { getApi } from './api';
import { installKeys } from './input/keys';
import { installPointer } from './input/pointer';
import { mountApp, type AppCtx } from './screens/app';
import { runFirstRun } from './screens/firstrun';
import { installScale } from './ui/scale';

declare global {
  interface Window {
    /** 首屏已画出（探测等它） */
    __ready?: boolean;
  }
}

/** 等到下一帧 */
function nextFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function boot(): Promise<void> {
  installKeys();
  installPointer();
  const stage = document.createElement('div');
  stage.id = 'stage';
  document.body.append(stage);
  installScale(stage);
  const api = getApi();
  let ctx: AppCtx | null = null;
  try {
    ctx = await mountApp(stage, api);
  } catch (e) {
    console.error('首页挂载失败', e);
  }
  // 等两帧确保首帧已画出，再通知 Rust 显示窗口，避免白屏闪一下；首页数据不等（getHome 被拒也照样显示）
  await nextFrame();
  await nextFrame();
  try {
    await api.uiReady();
  } finally {
    window.__ready = true;
  }
  if (ctx) runFirstRun(ctx).catch((e) => console.error('首次运行出错', e));
}

boot().catch((e) => console.error('ui_ready 调用失败', e));
