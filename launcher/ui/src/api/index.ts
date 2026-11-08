// 选接口：URL 带 ?fake=<剧本名> 时用假接口（探测用，&dev=1 打开开发模式，&nodir=1 让选目录返回 null），否则用真实的 Tauri 接口
import { createFakeApi } from './fake/index';
import { createTauriApi } from './tauri';
import type { Api } from './types';

export type * from './types';

let api: Api | null = null;

export function getApi(): Api {
  if (!api) {
    const q = new URLSearchParams(location.search);
    const fake = q.get('fake');
    api = fake ? createFakeApi(fake, { devMode: q.get('dev') === '1', noGameDir: q.get('nodir') === '1', slowListen: q.get('slowlisten') === '1', q }) : createTauriApi();
  }
  return api;
}
