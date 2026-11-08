// 启动过场探测：四步顺序、Esc 取消的时机、游戏仍在载入、失败横带的文案与动作（任务书表格逐行）、只由 launch-view 驱动
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import {
  DIALOG_OPEN, LMSG, MODE, SELECTED, STEP_ERR, STEP_RUN, TOAST, VEIL_ON, answer, calls, clickAt, dialog, emulateMotion, page, shot,
  withEndShots, waitDialog, waitHome,
} from './screen-helpers.mjs';

const OK_JOB = { skipBackup: false, ignoreVersion: false };
const ACT3 = ['重试', '跳过这次备份', '返回'];
const ACT2 = ['重试', '返回'];
const HINTS = `(document.querySelector('#home .lkeys').textContent)`;
const HINT_SEL = `#home .lkeys [data-k="Enter"]`;
/** 可点的「结束 me3 和游戏进程」提示在 */
const HINT_END = `(document.querySelector('${HINT_SEL}')?.textContent === '结束 me3 和游戏进程')`;
/** 假剧本推过的「游戏进程已出现」的 Waking 个数 */
const LOADING_VIEWS = `(window.__views || []).filter((v) => v.phase === 'waking' && v.slow && v.gameLoading).length`;

/** 页面里记下画面每次变化（步骤 → 唤起那行字 → 渐黑），不受 CDP 轮询间隔影响 */
const RECORD = `(() => {
  window.__seen = [];
  const state = () => document.querySelector('#home .veil.on') ? '渐黑'
    : (document.querySelector('#home .lmsg.on')?.textContent || document.querySelector('#home .step.run .n')?.textContent || '');
  const note = () => { const s = state(); if (s && s !== window.__seen.at(-1)) window.__seen.push(s); };
  new MutationObserver(note).observe(document.getElementById('home'), { subtree: true, childList: true, attributes: true, characterData: true });
  return true;
})()`;

const callCount = (name) => `(window.__calls || []).filter((c) => c.name === '${name}').length`;

/** 打开剧本、等首页状态画好、按 Enter 开始，返回失败横带 */
async function startAndFail(p, base, script, extra = '') {
  await p.open(base + page(script, extra));
  await waitHome(p);
  await p.key('Enter');
  return waitDialog(p, 4000, `${script}${extra} 没有弹出失败横带`);
}

// [剧本, URL 附加, 失败的那一步, 文案（填好 fake.ts 的样例）, 动作, 默认动作（不写 = 第一个）]
const PREFLIGHT_ROWS = [
  ['launch-fail-packNotFound', '', '预检', '找不到整合包：请把启动器放在整合包目录里运行', ['返回']],
  ['launch-fail-gameNotFound', '', '预检', '找不到游戏：整合包上级目录和 Steam 库里都没有 nightreign.exe', ['重新指定游戏所在文件夹', '整理问题记录', '返回']],
  ['launch-fail-gameVersionMismatch', '', '预检',
    '游戏版本和整合包不符（实际 1.3.4.0 / 118835320，需要 1.3.3.0 / 118835320），游戏更新后要等整合包更新', ['返回']],
  ['launch-fail-gameVersionMismatch', '&dev=1', '预检',
    '游戏版本和整合包不符（实际 1.3.4.0 / 118835320，需要 1.3.3.0 / 118835320），游戏更新后要等整合包更新', ['仍然启动', '返回'],
    // 开发模式多出的「仍然启动」不作默认（修复轮 M3）
    '返回'],
  ['launch-fail-me3FilesMissing', '', '预检', '缺少 me3 或模组文件：bin\\me3.exe、bin\\me3_mod_host.dll，请重新解压整合包', ['整理问题记录', '返回'], '返回'],
  ['launch-fail-steamNotRunning', '', '预检', 'Steam 没有运行，请先打开 Steam 并登录', ACT2],
  ['launch-fail-gameAlreadyRunning', '', '预检', '游戏已经在运行', ['切到游戏', '返回']],
  ['launch-fail-suffixMismatch', '', '预检', '存档后缀不一致：褪色燃烧.me3 是 co2，无缝联机是 sl2，两边会读写不同的存档', ['一键对齐后继续', '返回']],
];
const BACKUP_ROWS = [
  ['launch-fail-saveBusy', '', '备份存档', '存档正被其他程序占用，备份失败', ACT3],
  ['launch-fail-accountUnknown', '', '备份存档', '无法确定用哪个 Steam 账号的存档（找到 2 个），请登录 Steam 后重试', ACT3],
  ['launch-fail-backup', '', '备份存档', '备份失败：拒绝访问。 (os error 5)', ACT3],
  ['launch-fail-configWrite', '', '载入配置', '配置写入失败：拒绝访问。 (os error 5)', ['重试', '整理问题记录', '返回']],
];
const WAKE_ACTS = ['整理问题记录', '重试', '返回'];
const WAKE_ROWS = [
  ['launch-fail-spawnFailed', '', '唤起游戏', '无法运行 me3：系统找不到指定的文件。 (os error 2)', WAKE_ACTS, '重试'],
  ['launch-fail-me3Exit', '', '唤起游戏', 'me3 启动失败（退出码 1）', WAKE_ACTS, '重试'],
  ['launch-fail-gameNeverStarted', '', '唤起游戏', 'me3 已退出，但游戏没有启动', WAKE_ACTS, '重试'],
  ['launch-fail-gameExitedEarly', '', '唤起游戏', '游戏在出现窗口前就退出了', WAKE_ACTS, '重试'],
];

async function checkRows(p, base, rows) {
  for (const [script, extra, step, msg, opts, def = opts[0]] of rows) {
    const d = await startAndFail(p, base, script, extra);
    const what = `${script}${extra}`;
    assert.equal(d.msg, msg, `${what} 的文案`);
    assert.deepEqual(d.opts, opts, `${what} 的动作`);
    assert.equal(d.opts[d.sel], def, `${what} 的默认动作`);
    assert.ok(d.icon, `${what} 应带失败图标`);
    assert.equal(await p.eval(STEP_ERR), step, `${what} 失败的那一步`);
    if (script === 'launch-fail-gameNotFound' || script === 'launch-fail-me3Exit') await shot(p, `launch_fail_${script.slice(12)}`);
  }
}

export const tests = withEndShots({
  async launch_steps_in_order(p, { base }) {
    await emulateMotion(p);
    await p.open(base + page('launch-ok'));
    await waitHome(p);
    await p.eval(RECORD);
    await p.key('Enter');
    await p.waitFor(`${MODE} === 'launch'`, 2000, '没有进入启动过场');
    assert.deepEqual(await p.eval(`[...document.querySelectorAll('#home .step .n')].map((n) => n.textContent)`),
      ['预检', '备份存档', '载入配置', '唤起游戏']);
    await p.waitFor(`${STEP_RUN} === '载入配置'`, 3000, '没走到载入配置');
    // 等柔光从上一步移过来再截
    await sleep(350);
    await shot(p, 'launch_steps');
    await p.waitFor(VEIL_ON, 4000, '游戏窗口出现后没有渐黑');
    assert.deepEqual(await p.eval('window.__seen'), ['预检', '备份存档', '载入配置', '唤起游戏', '游戏载入中', '渐黑']);
  },

  async launch_cancel_before_wake(p, { base }) {
    await p.open(base + page('launch-ok'));
    await waitHome(p);
    await p.key('Enter');
    await p.waitFor(`${STEP_RUN} === '备份存档'`, 3000, '没走到备份存档');
    assert.ok((await p.eval(HINTS)).includes('取消启动'), '唤起前应提示可以 Esc 取消');
    const homes = await p.eval(callCount('getHome'));
    await p.key('Escape');
    // 取消受理后、cancelled 到之前（假接口隔一拍才发）：Esc 提示换成「正在取消…」
    await p.waitFor(`${HINTS} === '正在取消…'`, 2000, '取消受理后应提示「正在取消…」');
    await p.waitFor(`${MODE} === 'menu'`, 2000, 'Esc 后没有回到首页');
    assert.equal((await calls(p, 'cancelLaunch')).length, 1);
    await p.waitFor(`${callCount('getHome')} > ${homes}`, 2000, '回到首页后应刷新');
    assert.equal(await p.eval(SELECTED), 1);
    await sleep(1500);
    assert.equal(await p.eval(MODE), 'menu', '取消后不应再进入过场');
    assert.ok(!(await p.eval(VEIL_ON)));
  },

  async launch_esc_ignored_during_wake(p, { base }) {
    await p.open(base + page('launch-ok'));
    await waitHome(p);
    await p.key('Enter');
    await p.waitFor(`${STEP_RUN} === '唤起游戏'`, 3000, '没走到唤起游戏');
    assert.equal(await p.eval(HINTS), '游戏正在唤起，已不能取消');
    await p.key('Escape');
    await p.click(150, 450, { button: 'right' });
    await sleep(200);
    assert.equal(await p.eval(MODE), 'launch', '唤起后 Esc / 右键不应返回');
    assert.equal((await calls(p, 'cancelLaunch')).length, 0, '唤起后不应再调 cancelLaunch');
    await p.waitFor(VEIL_ON, 3000, '启动应照常走到渐黑');
  },

  async launch_slow_shows_actions(p, { base }) {
    await emulateMotion(p);
    await p.open(base + page('launch-slow'));
    await waitHome(p);
    await p.key('Enter');
    let d = await waitDialog(p, 5000, '游戏仍在载入时没有给动作');
    await shot(p, 'launch_slow');
    assert.equal(d.msg, '游戏仍在载入');
    assert.deepEqual(d.opts, ['整理问题记录', '结束 me3 和游戏进程', '继续等待']);
    assert.equal(d.opts[d.sel], '继续等待', '默认应是不结束进程');
    // 继续等待：关掉提问，过场留着
    await answer(p, 'Enter');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, '继续等待应关掉提问');
    assert.equal(await p.eval(LMSG), '游戏仍在载入');
    assert.equal(await p.eval(MODE), 'launch');
    assert.equal((await calls(p, 'endProcesses')).length, 0);
    // 结束 me3 和游戏进程：cancelled 后回到首页
    await p.open(base + page('launch-slow'));
    await waitHome(p);
    await p.key('Enter');
    await waitDialog(p, 5000, '第二次没有给动作');
    await p.key('ArrowLeft');
    await answer(p, 'Enter');
    await p.waitFor(`${MODE} === 'menu'`, 2000, '结束进程后应回到首页');
    assert.equal((await calls(p, 'endProcesses')).length, 1);
    // 提问开着时游戏窗口出现：提问撤下，渐黑
    await p.open(base + page('launch-slow'));
    await waitHome(p);
    await p.key('Enter');
    await waitDialog(p, 5000, '第三次没有给动作');
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'running' })`);
    await p.waitFor(`!${DIALOG_OPEN} && ${VEIL_ON}`, 2000, '游戏窗口出现后提问应撤下并渐黑');
  },

  async launch_slow_end_hint_persists(p, { base }) {
    // launch-slow：2s 时（slow, 无游戏进程）弹提问；4.5s 时游戏进程出现（slow 不变）
    await p.open(base + page('launch-slow'));
    await waitHome(p);
    await p.key('Enter');
    await waitDialog(p, 5000, '游戏仍在载入时没有给动作');
    await answer(p, 'Enter');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, '继续等待应关掉提问');
    assert.equal(await p.eval(HINT_END), true, '继续等待后应留着可点的「结束 me3 和游戏进程」');
    assert.equal(await p.eval(LOADING_VIEWS), 0, '游戏进程出现之前就该看到提示（时序前提）');
    // 之后的 Waking：不重开提问，提示还在
    await p.waitFor(`${LOADING_VIEWS} > 0`, 4000, '假剧本没有推出游戏进程出现');
    await sleep(150);
    assert.ok(!(await p.eval(DIALOG_OPEN)), '之后的 Waking 不应重开提问');
    assert.equal(await p.eval(HINT_END), true, '之后的 Waking 不应去掉提示');
    // 点提示 → 同一个提问（默认继续等待）
    await clickAt(p, `${HINT_SEL}`);
    let d = await waitDialog(p, 2000, '点提示应打开提问');
    assert.deepEqual(d.opts, ['整理问题记录', '结束 me3 和游戏进程', '继续等待']);
    assert.equal(d.sel, 2);
    // 提问开着时又来 Waking：不关、不重开（选中项不被重置）
    await p.key('ArrowLeft');
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'waking', slow: true, gameLoading: true })`);
    await sleep(150);
    d = await dialog(p);
    assert.ok(d && d.sel === 1, '提问开着时的 Waking 不应关掉或重开提问');
    await answer(p, 'Enter');
    await p.waitFor(`${callCount('endProcesses')} === 1`, 2000, '选「结束」应调 endProcesses');
    await p.waitFor(`${MODE} === 'menu'`, 2000, '结束进程后应回到首页');
  },

  async launch_running_veil_focus(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'wake' })`);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'running' })`);
    await p.waitFor(VEIL_ON, 2000, 'running 应渐黑');
    assert.equal(await p.eval(`document.querySelector('#home .veil .vmsg').textContent`), '游戏运行中');
    // 切页后的点击冷却过了再点
    await sleep(600);
    await clickAt(p, '#home .veil [data-k="Enter"]');
    await p.waitFor(`${callCount('focusGame')} === 1`, 2000, '点「切到游戏」应调 focusGame');
    // 其余键照旧吞掉
    await p.key('Escape');
    await p.key('ArrowUp');
    await sleep(150);
    assert.ok(await p.eval(VEIL_ON));
    assert.equal(await p.eval(MODE), 'launch');
    assert.equal((await calls(p, 'cancelLaunch')).length, 0);
    assert.equal((await calls(p, 'focusGame')).length, 1);
  },

  async launch_layer_swap_guards_clicks(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    // 菜单 → 过场：500ms 内的点击丢弃
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'backup' })`);
    await clickAt(p, '#home .lkeys [data-k="Escape"]');
    await sleep(100);
    assert.equal((await calls(p, 'cancelLaunch')).length, 0, '切到过场后 500ms 内的点击应丢弃');
    await sleep(500);
    await clickAt(p, '#home .lkeys [data-k="Escape"]');
    await p.waitFor(`${callCount('cancelLaunch')} === 1`, 2000, '冷却过后点 Esc 提示应取消');
    // 过场 → 菜单：同样，单击菜单项（会执行）和「确定」提示都丢弃
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'cancelled' })`);
    await p.waitFor(`${MODE} === 'menu'`, 2000);
    await clickAt(p, '#home .tmenu button[data-i="0"]');
    await clickAt(p, '#home .hkeys [data-k="Enter"]');
    await sleep(100);
    assert.equal((await calls(p, 'startGame')).length, 0, '回到菜单后 500ms 内的点击应丢弃');
    await sleep(500);
    await clickAt(p, '#home .hkeys [data-k="Enter"]');
    await p.waitFor(`${callCount('startGame')} === 1`, 2000, '冷却过后点「确定」应开始游戏');
  },

  async launch_unknown_failure_goes_home(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'preflight' })`);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'failed', step: 'preflight', reason: { kind: 'bogus' } })`);
    await p.waitFor(`${MODE} === 'menu'`, 2000, '不认识的失败原因应回到首页');
    assert.equal(await p.eval(TOAST), '启动失败，原因未知');
    assert.ok(!(await p.eval(DIALOG_OPEN)));
  },

  async 'launch_failures_map_actions[preflight]'(p, { base }) {
    await checkRows(p, base, PREFLIGHT_ROWS);
  },

  async 'launch_failures_map_actions[backup+config]'(p, { base }) {
    await checkRows(p, base, BACKUP_ROWS);
  },

  async 'launch_failures_map_actions[wake]'(p, { base }) {
    await checkRows(p, base, WAKE_ROWS);
  },

  async launch_failure_actions_restart(p, { base }) {
    // 重试 = startGame(false, false)：同一剧本再失败一次
    await startAndFail(p, base, 'launch-fail-steamNotRunning');
    await answer(p, 'Enter');
    await p.waitFor(`${callCount('startGame')} === 2`, 2000, '重试没有再开始');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000);
    await waitDialog(p, 3000, '重试后应再次失败');
    assert.deepEqual((await calls(p, 'startGame')).map((c) => c.args), [OK_JOB, OK_JOB]);
    // 跳过这次备份 = startGame(skipBackup)：这次走到渐黑
    await startAndFail(p, base, 'launch-fail-saveBusy');
    await p.key('ArrowRight');
    await answer(p, 'Enter');
    await p.waitFor(VEIL_ON, 4000, '跳过备份后应走到渐黑');
    assert.deepEqual((await calls(p, 'startGame'))[1].args, { skipBackup: true, ignoreVersion: false });
    // 仍然启动 = startGame(ignoreVersion)，只在开发模式
    await startAndFail(p, base, 'launch-fail-gameVersionMismatch', '&dev=1');
    await p.key('ArrowLeft');
    await answer(p, 'Enter');
    await p.waitFor(VEIL_ON, 4000, '仍然启动后应走到渐黑');
    assert.deepEqual((await calls(p, 'startGame'))[1].args, { skipBackup: false, ignoreVersion: true });
  },

  async launch_retry_rejected_stays_on_failure(p, { base }) {
    // 未知剧本：startGame 必被拒。失败横带上点「重试」被拒后只提示，留在失败画面（裁定 A）
    await p.open(base + page('nope'));
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'failed', step: 'preflight',
      reason: { kind: 'preflight', issue: { kind: 'steamNotRunning', blocking: true, detail: '' } } })`);
    let d = await waitDialog(p, 2000, '失败横带没有出现');
    assert.deepEqual(d.opts, ACT2);
    // getHome 被拒的那条提示先收掉，下面看到的只能是重试被拒的
    await p.eval(`document.querySelector('.toast')?.classList.remove('on')`);
    await answer(p, 'Enter');
    await p.waitFor(`${TOAST}.includes('未知的假剧本')`, 2000, '重试被拒应提示原因');
    d = await waitDialog(p, 2000, '重试被拒后应回到失败横带');
    assert.equal(d.msg, 'Steam 没有运行，请先打开 Steam 并登录');
    assert.equal(await p.eval(MODE), 'launch');
    assert.equal(await p.eval(STEP_ERR), '预检');
    assert.equal((await calls(p, 'startGame')).length, 1);
  },

  async launch_failure_actions_leave(p, { base }) {
    // 返回：回到首页菜单并刷新
    await startAndFail(p, base, 'launch-fail-configWrite');
    let homes = await p.eval(callCount('getHome'));
    await answer(p, 'Escape');
    await p.waitFor(`${MODE} === 'menu'`, 2000, '返回后应回到首页');
    await p.waitFor(`${callCount('getHome')} > ${homes}`, 2000);
    assert.ok(!(await p.eval(DIALOG_OPEN)));
    assert.equal(await p.eval(SELECTED), 1);
    // 切到游戏 = focusGame
    await startAndFail(p, base, 'launch-fail-gameAlreadyRunning');
    await answer(p, 'Enter');
    await p.waitFor(`${MODE} === 'menu'`, 2000, '切到游戏后应回到首页');
    assert.equal((await calls(p, 'focusGame')).length, 1);
    // 重新指定游戏所在文件夹 = chooseGameDir 后刷新首页
    await startAndFail(p, base, 'launch-fail-gameNotFound');
    homes = await p.eval(callCount('getHome'));
    await answer(p, 'Enter');
    await p.waitFor(`${MODE} === 'menu'`, 2000, '重新指定后应回到首页');
    assert.equal((await calls(p, 'chooseGameDir')).length, 1);
    await p.waitFor(`${callCount('getHome')} > ${homes}`, 2000, '重新指定后应刷新首页');
    const order = await p.eval(`window.__calls.map((c) => c.name).filter((n) => n === 'chooseGameDir' || n === 'getHome').slice(-2)`);
    assert.deepEqual(order, ['chooseGameDir', 'getHome'], '先选目录再刷新');
  },

  async launch_choose_dir_rejected_toast(p, { base }) {
    const MSG = '这个文件夹里没有游戏，请选择含 Game\\nightreign.exe 的游戏根目录';
    // 首页横带动作：选的文件夹里没有游戏 = 提示
    await p.open(base + page('home-nogame', '&nodir=1'));
    await waitHome(p);
    await p.key('ArrowUp');
    await p.key('Enter');
    await p.waitFor(`${TOAST} === ${JSON.stringify(MSG)}`, 2000, '首页横带选目录被拒应提示');
    assert.equal((await calls(p, 'chooseGameDir')).length, 1);
    // 失败横带动作：同一条提示
    await startAndFail(p, base, 'launch-fail-gameNotFound', '&nodir=1');
    await answer(p, 'Enter');
    await p.waitFor(`${MODE} === 'menu'`, 2000, '选目录后应回到首页');
    await p.waitFor(`${TOAST} === ${JSON.stringify(MSG)}`, 2000, '失败横带选目录被拒应提示');
  },

  async launch_view_driven(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    // 托盘发起：只收到 launch-view，没调 startGame 也要进入过场
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'backup' })`);
    await p.waitFor(`${MODE} === 'launch' && ${STEP_RUN} === '备份存档'`, 2000, 'step 应切到启动过场');
    assert.equal(await p.eval(`document.querySelector('#home .step.ok .n')?.textContent`), '预检', '之前的步骤应显示完成');
    assert.equal((await calls(p, 'startGame')).length, 0);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'waking', slow: false, gameLoading: true })`);
    await p.waitFor(`${LMSG} === '游戏载入中'`, 2000, 'gameLoading 应写「游戏载入中」');
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'running' })`);
    await p.waitFor(VEIL_ON, 2000, 'running 应渐黑');
    await sleep(900);
    await shot(p, 'launch_running');
    const homes = await p.eval(callCount('getHome'));
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'finished', abnormal: false })`);
    await p.waitFor(`${MODE} === 'menu' && !${VEIL_ON}`, 2000, 'finished 应回到首页');
    await p.waitFor(`${callCount('getHome')} > ${homes}`, 2000, 'finished 后应刷新首页');
    // 失败横带开着时托盘又开始一次：横带撤下，过场从头画
    await p.open(base + page('launch-fail-steamNotRunning'));
    await waitHome(p);
    await p.key('Enter');
    await waitDialog(p, 3000, '没有失败横带');
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'preflight' })`);
    await p.waitFor(`!${DIALOG_OPEN} && ${STEP_RUN} === '预检' && ${STEP_ERR} === ''`, 2000, '新的启动应撤下失败横带并从头画');
  },
});
