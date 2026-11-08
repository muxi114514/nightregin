// 探测入口：起 npm run preview（4173，strictPort）和无头 Edge，依次跑 probes\*.probe.mjs 里的各项，打印 PASS / FAIL；有失败以 1 退出。
// 用法：node probes\run.mjs [过滤名]（按「文件名/项目名」子串筛选）。无论成败都按 pid 结束自己起的进程树并删掉临时用户目录
import { spawn } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { closeAllBrowsers, killTree, launchEdge, sleep } from './cdp.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const uiDir = dirname(here);
const outDir = join(here, 'out');
const PORT = 4173;
const BASE = `http://localhost:${PORT}/`;
const TEST_TIMEOUT_MS = 30000;
const PREVIEW_TIMEOUT_MS = 30000;
const ANSI = /\x1b\[[0-9;]*m/g;
const filter = process.argv[2] ?? '';

let preview = null;
let browser = null;
/** 预览进程是否已经退出（退出后不再 taskkill） */
let previewExited = false;
/** 收尾没收干净（临时目录删不掉）时整次运行判失败 */
let dirty = false;

/** 同步收尾：成功、失败、异常、信号都走这里；起过的 Edge 都在 cdp.mjs 的登记表里，连接阶段被打断也收得到 */
function cleanup() {
  for (const { profile, swept, removed } of closeAllBrowsers()) {
    if (swept.length) console.log(`已补结束残留的 Edge 进程：${swept.join(', ')}`);
    if (!removed) {
      console.log(`临时用户目录没删掉：${profile}`);
      dirty = true;
    }
  }
  browser = null;
  // 已退出的进程 pid 可能被系统复用，不能再杀
  if (preview && !previewExited) killTree(preview.pid);
  preview = null;
}
process.on('exit', cleanup);
for (const sig of ['SIGINT', 'SIGTERM', 'SIGBREAK']) {
  process.on(sig, () => {
    cleanup();
    process.exit(130);
  });
}

async function collect() {
  const files = readdirSync(here).filter((f) => f.endsWith('.probe.mjs')).sort();
  const picked = [];
  for (const file of files) {
    const mod = await import(pathToFileURL(join(here, file)).href);
    const group = file.replace(/\.probe\.mjs$/, '');
    for (const [name, fn] of Object.entries(mod.tests ?? {})) {
      const id = `${group}/${name}`;
      if (!filter || id.includes(filter)) picked.push({ id, fn });
    }
  }
  return picked;
}

/** GET 一次的状态码，1 秒没回应或连不上为 0 */
async function status(url) {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(1000) });
    await res.body?.cancel();
    return res.status;
  } catch {
    return 0;
  }
}

/**
 * 起 npm run preview，轮询到 probe.html 返回 200 才算起来，不看输出（Windows 上即使输出被管道接走，端口号也会带颜色码）。
 * 起之前端口已有服务、或进程先退出（例如端口被占）都报错，免得探到别人的服务
 */
async function startPreview() {
  if (await status(BASE)) throw new Error(`端口 ${PORT} 上已经有服务在跑，不能探测`);
  const proc = spawn(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', 'npm run preview'], {
    cwd: uiDir, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, NO_COLOR: '1' },
  });
  preview = proc;
  let log = '';
  let exited = null;
  const onData = (chunk) => { log += chunk.replace(ANSI, ''); };
  proc.stdout.setEncoding('utf8').on('data', onData);
  proc.stderr.setEncoding('utf8').on('data', onData);
  proc.on('error', (e) => { exited = e.message; });
  proc.on('exit', (code) => { exited = `退出码 ${code}`; previewExited = true; });
  for (const end = Date.now() + PREVIEW_TIMEOUT_MS; Date.now() < end;) {
    if (exited !== null) throw new Error(`预览服务退出了（${exited}），端口 ${PORT} 可能被占用：\n${log}`);
    if ((await status(`${BASE}probe.html`)) === 200) return;
    await sleep(200);
  }
  throw new Error(`预览服务 ${PREVIEW_TIMEOUT_MS / 1000} 秒没起来：\n${log}`);
}

function withTimeout(promise, ms, what) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${what} 超时`)), ms); }),
  ]).finally(() => clearTimeout(timer));
}

async function runOne({ id, fn }) {
  const page = await browser.newPage();
  try {
    await withTimeout(fn(page, { base: BASE }), TEST_TIMEOUT_MS, id);
    if (page.errors.length) throw new Error(`页面有未捕获的异常：${page.errors.join(' | ')}`);
    console.log(`PASS ${id}`);
    return true;
  } catch (e) {
    console.log(`FAIL ${id}：${e.message}`);
    await page.screenshot(join(outDir, `${id.replace('/', '_')}.fail.png`)).catch(() => {});
    return false;
  } finally {
    await page.close();
  }
}

async function main() {
  const tests = await collect();
  if (!tests.length) {
    // 退出码仍为 0：test.ps1 把同一个过滤名也交给 cargo，可能只有一边匹配得上
    console.log(`!!!!!!!! 警告：没有匹配「${filter}」的探测，一项探测都没跑 !!!!!!!!`);
    return 0;
  }
  await startPreview();
  browser = await launchEdge();
  let failed = 0;
  for (const t of tests) {
    if (!(await runOne(t))) failed++;
  }
  console.log(`探测：${tests.length - failed} 通过，${failed} 失败`);
  return failed ? 1 : 0;
}

let code = 1;
try {
  code = await main();
} catch (e) {
  console.log(`探测中断：${e.message}`);
} finally {
  await browser?.quit();
  cleanup();
}
process.exit(dirty ? 1 : code);
