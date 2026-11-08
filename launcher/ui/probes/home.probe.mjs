// 首页探测：在 index.html 的假剧本上发真实 CDP 键鼠，核对菜单、状态行、横带、关窗确认、可点的「确定」和动画暂停
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import {
  BANNER, DIALOG_OPEN, MODE, SELECTED, STATUS, TOAST, VEIL_ON, answer, calls, clickAt, dialog, emulateMotion, page, settle, shot, withEndShots,
  waitDialog, waitHome,
} from './screen-helpers.mjs';

const ITEM = (n) => `.tmenu button[data-i="${n - 1}"]`;
const TAIL = '#home .issue .opt';
const ENTER_HINT = '#home .hkeys [data-k="Enter"]';
/** 菜单和横带之外的空地 */
const EMPTY = { x: 150, y: 450 };
const OK_JOB = { skipBackup: false, ignoreVersion: false };
const callCount = (name) => `(window.__calls || []).filter((c) => c.name === '${name}').length`;
/** 页面里记下每次 click 的 detail（1 单击、2 双击的第二下、0 键盘合成） */
const RECORD_CLICKS = `(() => { window.__details = []; addEventListener('click', (e) => window.__details.push(e.detail), true); return true; })()`;

/** 真实的双击：同一点按下松开两次，第二次 clickCount 为 2（click 的 detail 随之为 2） */
async function doubleClickAt(p, selector) {
  const c = await p.center(selector);
  await p.mouseMove(c.x, c.y);
  for (const clickCount of [1, 2]) {
    const ev = { x: c.x, y: c.y, button: 'left', clickCount };
    await p.send('Input.dispatchMouseEvent', { ...ev, type: 'mousePressed', buttons: 1 });
    await p.send('Input.dispatchMouseEvent', { ...ev, type: 'mouseReleased', buttons: 0 });
  }
}

const homeState = (o) => JSON.stringify({ version: null, deepNight: { enabled: true, level: 5 }, issues: [], lowEffects: false, devMode: false, ...o });
const emitHome = (p, o) => p.eval(`window.__fakeEmit('home-state', ${homeState(o)})`);
const issue = (kind, detail = '', blocking = true) => ({ kind, blocking, detail });
/** html 是否暂停、余烬是否换成静帧 */
const MOTION = `({ paused: document.documentElement.classList.contains('paused'), still: document.querySelector('.bg img').src.endsWith('/assets/ember_still.png') })`;

export const tests = withEndShots({
  async hints_double_click_runs_once(p, { base }) {
    await p.open(base + page('launch-hold'));
    await waitHome(p);
    await sleep(600);
    await doubleClickAt(p, ENTER_HINT);
    await sleep(250);
    assert.equal((await calls(p, 'startGame')).length, 1, '双击按键提示只开始一次');
  },

  async early_launch_view_not_lost(p, { base }) {
    await p.open(base + page('launch-early', '&slowlisten=1'));
    await p.waitFor(`${MODE} === 'launch'`, 3000, '订阅还没注册好就报告就绪，第一个进度丢了');
  },

  async home_menu_order_and_disabled(p, { base }) {
    await emulateMotion(p);
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await settle(p);
    await shot(p, 'home_menu');
    const items = await p.eval(`[...document.querySelectorAll('.tmenu button')].map((b) => [b.textContent, b.getAttribute('aria-disabled')])`);
    assert.deepEqual(items.map((i) => i[0]), ['开始游戏', '图鉴', '深夜与联机', '设置', '存档', '更新']);
    assert.deepEqual(items.map((i) => i[1]), ['false', 'false', 'false', 'false', 'false', 'true'], '更新还不可用');
    assert.equal(await p.eval(SELECTED), 1, '初始应选中第 1 项');
    await p.key('ArrowDown');
    assert.equal(await p.eval(SELECTED), 2, '图鉴可用，↓ 应选中它');
    await p.key('ArrowUp');
    for (let i = 0; i < 4; i++) await p.key('ArrowDown');
    assert.equal(await p.eval(SELECTED), 5);
    await p.key('ArrowDown');
    assert.notEqual(await p.eval(SELECTED), 6, '↓ 应跳过不可用的项');
    const before = await p.eval(SELECTED);
    const c = await p.center(ITEM(6));
    await p.mouseMove(c.x, c.y);
    await sleep(150);
    assert.equal(await p.eval(SELECTED), before, '悬停不可用的项不应选中它');
    await p.click(c.x, c.y);
    await sleep(150);
    assert.equal(await p.eval(SELECTED), before, '单击不可用的项不应选中它');
    assert.equal((await calls(p, 'startGame')).length, 0, '单击不可用的项不应执行');
  },

  async home_status_texts(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    assert.deepEqual(await p.eval(STATUS), ['版本未知 · 请导入完整包', '深夜 5 层', '一切就绪']);
    assert.equal(await p.eval(BANNER), null, '没有问题时不显示横带');
    // 计划 1 版本段和深夜段不响应点击
    assert.equal(await p.eval(`document.querySelectorAll('#home .status button').length`), 2, '深夜段和就绪段可点，版本段计划 4 再接');
    await emitHome(p, { version: '1.9.4', deepNight: { enabled: false, level: 5 } });
    assert.deepEqual(await p.eval(STATUS), ['1.9.4', '深夜关闭', '一切就绪']);
  },

  async home_banner_shows_top_issue(p, { base }) {
    await emulateMotion(p);
    await p.open(base + page('home-nogame'));
    await waitHome(p);
    await settle(p);
    await shot(p, 'home_banner');
    assert.deepEqual(await p.eval(BANNER), {
      msg: '找不到游戏：整合包上级目录和 Steam 库里都没有 nightreign.exe', action: '重新指定游戏所在文件夹',
    });
    assert.equal((await p.eval(STATUS)).at(-1), '找不到游戏', '就绪段写横带那条的简短名');
    // 横带只显示 issues[0]
    await emitHome(p, { issues: [issue('gameVersionMismatch', '实际 1.3.4.0 / 118835320，需要 1.3.3.0 / 118835320'), issue('lastExitAbnormal', '', false)] });
    assert.equal((await p.eval(BANNER)).msg, '游戏版本和整合包不符（实际 1.3.4.0 / 118835320，需要 1.3.3.0 / 118835320），游戏更新后要等整合包更新');
    assert.equal((await p.eval(STATUS)).at(-1), '游戏版本和整合包不符');
    // 回到找不到游戏：↓ 跳过不可用项落到横带动作，Enter 重新指定后刷新首页
    await emitHome(p, { issues: [issue('gameNotFound')] });
    await p.key('ArrowUp');
    assert.equal(await p.eval(SELECTED), 'tail', '↑ 应绕到横带动作');
    const before = (await calls(p, 'getHome')).length;
    await p.key('Enter');
    await p.waitFor(`(window.__calls || []).filter((c) => c.name === 'getHome').length > ${before}`, 2000, '重新指定后没有刷新首页');
    assert.equal((await calls(p, 'chooseGameDir')).length, 1);
    await p.waitFor(`${BANNER} === null`, 2000, '指定好游戏目录后横带应消失');
    assert.equal((await p.eval(STATUS)).at(-1), '一切就绪');
    assert.equal(await p.eval(SELECTED), 1);
  },

  async home_banner_rows_follow_table(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    // [问题, 开发模式, 横带文案, 横带动作]
    const rows = [
      [issue('packNotFound'), false, '找不到整合包：请把启动器放在整合包目录里运行', null],
      [issue('me3FilesMissing', 'bin\\me3.exe'), false, '缺少 me3 或模组文件：bin\\me3.exe，请重新解压整合包', '整理问题记录'],
      [issue('suffixMismatch', '褪色燃烧.me3 是 co2，无缝联机是 sl2'), false, '存档后缀不一致：褪色燃烧.me3 是 co2，无缝联机是 sl2，两边会读写不同的存档', '一键改正'],
      [issue('chanceSetterSuffix', '.CL_SAVE', false), false, '配置不一致：ChanceSetter 的存档后缀是 .CL_SAVE，夜王防重复和地图记录不会生效', '一键改正'],
      [issue('lastExitAbnormal', '', false), false, '上次游戏异常退出', '整理问题记录'],
      [issue('gameAlreadyRunning', 'pid 4242'), false, '游戏已经在运行', '切到游戏'],
      [issue('gameVersionMismatch', 'x'), false, '游戏版本和整合包不符（x），游戏更新后要等整合包更新', null],
      [issue('gameVersionMismatch', 'x'), true, '游戏版本和整合包不符（x），游戏更新后要等整合包更新', '仍然启动'],
    ];
    for (const [it, devMode, msg, action] of rows) {
      await emitHome(p, { issues: [it], devMode });
      assert.deepEqual(await p.eval(BANNER), { msg, action }, `${it.kind}${devMode ? '（开发模式）' : ''}`);
    }
    // 开发模式的「仍然启动」= startGame(ignoreVersion)
    await p.key('ArrowUp');
    await p.key('Enter');
    await sleep(100);
    assert.deepEqual((await calls(p, 'startGame')).map((c) => c.args), [{ skipBackup: false, ignoreVersion: true }]);
  },

  async home_close_confirms_during_launch(p, { base }) {
    // launch-hold：开始后停在备份那一步，不靠和假接口计时器抢时间
    await p.open(base + page('launch-hold'));
    await waitHome(p);
    // 没在启动：× 直接关闭
    await clickAt(p, '.win .x');
    await sleep(150);
    assert.equal((await calls(p, 'closeApp')).length, 1, '菜单页点 × 应直接关闭');
    assert.ok(!(await p.eval(DIALOG_OPEN)), '菜单页点 × 不应确认');
    // 启动中点 ×：先确认，默认「取消」
    await p.key('Enter');
    await p.waitFor(`${MODE} === 'launch'`, 2000, '没有进入启动过场');
    await clickAt(p, '.win .x');
    const d = await waitDialog(p, 2000, '启动中点 × 没有弹确认');
    await shot(p, 'home_close_confirm');
    assert.deepEqual(d.opts, ['关闭启动器', '取消']);
    assert.equal(d.opts[d.sel], '取消', '默认应选中「取消」');
    await answer(p, 'Enter');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, 'Enter 应关掉确认框');
    assert.equal((await calls(p, 'closeApp')).length, 1, '选「取消」不应关闭');
    assert.equal(await p.eval(MODE), 'launch', '取消后启动照常进行');
    // 确认框开着时游戏窗口出现：「启动还没有完成」已过时，撤下
    await sleep(600);
    await clickAt(p, '.win .x');
    await waitDialog(p, 2000, '第二次点 × 没有弹确认');
    assert.equal(await p.eval(MODE), 'launch', '应还停在启动过场');
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'running' })`);
    await p.waitFor(`${VEIL_ON} && !${DIALOG_OPEN}`, 2000, '游戏窗口出现后确认框应撤下');
    assert.equal((await calls(p, 'closeApp')).length, 1);
  },

  async home_close_requested_confirms(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    // 托盘或其他途径发起的启动（只有 launch-view），停在备份那一步
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'backup' })`);
    await p.waitFor(`${MODE} === 'launch'`, 2000, '没有进入启动过场');
    await p.eval(`window.__fakeEmit('close-requested')`);
    let d = await waitDialog(p, 2000, 'close-requested 没有弹确认');
    assert.equal(d.opts[d.sel], '取消', '默认应选中「取消」');
    await answer(p, 'Escape');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, 'Esc 应取消');
    assert.equal((await calls(p, 'closeApp')).length, 0, '取消后不应关闭');
    await sleep(600);
    await p.eval(`window.__fakeEmit('close-requested')`);
    d = await waitDialog(p, 2000, '第二次 close-requested 没有弹确认');
    await p.key('ArrowLeft');
    assert.equal((await dialog(p)).sel, 0);
    await answer(p, 'Enter');
    await p.waitFor(`(window.__calls || []).some((c) => c.name === 'closeApp')`, 2000, '确定后应调 closeApp');
  },

  async home_click_executes(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    // 单击「开始游戏」：选中并执行，同 Enter
    await clickAt(p, ITEM(1));
    await p.waitFor(`${callCount('startGame')} === 1`, 2000, '单击「开始游戏」应开始游戏');
    assert.deepEqual((await calls(p, 'startGame'))[0].args, OK_JOB);
    // 单击横带动作（开发模式的「仍然启动」）：执行的是它
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await emitHome(p, { issues: [issue('gameVersionMismatch', 'x')], devMode: true });
    await clickAt(p, TAIL);
    await p.waitFor(`${callCount('startGame')} === 1`, 2000, '单击横带动作应执行「仍然启动」');
    assert.deepEqual((await calls(p, 'startGame')).map((c) => c.args), [{ skipBackup: false, ignoreVersion: true }]);
    // 单击过的项：鼠标移开后光标留在它上面（选目录被拒，横带还在）
    await p.open(base + page('home-nogame', '&nodir=1'));
    await waitHome(p);
    await clickAt(p, TAIL);
    await p.waitFor(`${callCount('chooseGameDir')} === 1`, 2000, '单击横带动作应执行它');
    await p.waitFor(`${TOAST} !== ''`, 2000, '选目录被拒应提示');
    assert.equal(await p.eval(SELECTED), 'tail');
    await p.mouseMove(EMPTY.x, EMPTY.y);
    await sleep(100);
    assert.equal(await p.eval(SELECTED), 'tail', '单击过的项移开鼠标后应保留');
    assert.equal((await calls(p, 'startGame')).length, 0);
  },

  async home_double_click_runs_once(p, { base }) {
    // 未知剧本：startGame 被拒、不进过场，层不变，500ms 冷却拦不到双击的第二下
    await p.open(base + page('nope'));
    await p.waitFor(`${TOAST}.includes('未知的假剧本')`, 2000, 'getHome 被拒应提示');
    await p.eval(RECORD_CLICKS);
    await doubleClickAt(p, ITEM(1));
    await sleep(300);
    assert.deepEqual(await p.eval('window.__details'), [1, 2], '应发出真实的双击');
    assert.equal((await calls(p, 'startGame')).length, 1, '双击「开始游戏」只应开始一次');
    // 单击后按钮留着焦点：空格在它上面合成的点击（detail 0）也不执行
    await p.key(' ');
    await sleep(150);
    assert.deepEqual(await p.eval('window.__details'), [1, 2, 0], '空格应在按钮上合成一次点击');
    assert.equal((await calls(p, 'startGame')).length, 1, '空格合成的点击不应执行');
    // 横带动作「重新指定」：双击只弹一次选目录（选目录被拒，横带还在、层不变）
    await p.open(base + page('home-nogame', '&nodir=1'));
    await waitHome(p);
    await p.eval(RECORD_CLICKS);
    await doubleClickAt(p, TAIL);
    await sleep(300);
    assert.deepEqual(await p.eval('window.__details'), [1, 2], '应发出真实的双击');
    assert.equal((await calls(p, 'chooseGameDir')).length, 1, '双击横带动作只应执行一次');
  },

  async home_enter_hint_starts_game(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    // 提示里的「确定」照样可点：执行当前选中的「开始游戏」
    await p.mouseMove(EMPTY.x, EMPTY.y);
    assert.equal(await p.eval(`document.querySelector('${ENTER_HINT}').textContent`), '确定');
    await clickAt(p, ENTER_HINT);
    await p.waitFor(`${callCount('startGame')} === 1`, 2000, '点「确定」提示应开始游戏');
    assert.deepEqual((await calls(p, 'startGame'))[0].args, OK_JOB);
    // 键盘选中横带动作（开发模式的「仍然启动」）后点「确定」：执行的是它
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await emitHome(p, { issues: [issue('gameVersionMismatch', 'x')], devMode: true });
    await p.key('ArrowUp');
    assert.equal(await p.eval(SELECTED), 'tail');
    await clickAt(p, ENTER_HINT);
    await p.waitFor(`${callCount('startGame')} === 1`, 2000, '点「确定」应执行「仍然启动」');
    assert.deepEqual((await calls(p, 'startGame')).map((c) => c.args), [{ skipBackup: false, ignoreVersion: true }],
      '执行的应是选中的「仍然启动」，不是「开始游戏」');
  },

  async home_pointer_leave_returns_to_start(p, { base }) {
    await p.open(base + page('home-nogame'));
    await waitHome(p);
    const t = await p.center(TAIL);
    await p.mouseMove(t.x, t.y);
    await p.waitFor(`${SELECTED} === 'tail'`, 2000, '悬停横带动作应选中它');
    await p.mouseMove(EMPTY.x, EMPTY.y);
    await p.waitFor(`${SELECTED} === 1`, 2000, '鼠标离开菜单后光标应回到「开始游戏」');
    // 鼠标本来就在菜单外：键盘选中的项不受鼠标移动影响
    await p.key('ArrowUp');
    await p.mouseMove(EMPTY.x + 20, EMPTY.y);
    await sleep(150);
    assert.equal(await p.eval(SELECTED), 'tail');
  },

  async home_motion_pauses(p, { base }) {
    await emulateMotion(p);
    await p.open(base + page('home-ok'));
    await waitHome(p);
    assert.ok(await p.eval(`document.querySelector('.bg img').src.endsWith('/assets/ember.webp')`), '平时应是动的余烬');
    assert.deepEqual(await p.eval(MOTION), { paused: false, still: false });
    await p.eval(`window.__fakeEmit('window-visibility', { visible: false })`);
    assert.deepEqual(await p.eval(MOTION), { paused: true, still: true }, '窗口不可见时应暂停并换静帧');
    await p.eval(`window.__fakeEmit('window-visibility', { visible: true })`);
    assert.deepEqual(await p.eval(MOTION), { paused: false, still: false }, '窗口可见后应恢复');
    await emitHome(p, { lowEffects: true });
    assert.deepEqual(await p.eval(MOTION), { paused: true, still: true }, '低特效应暂停并换静帧');
    await emitHome(p, { lowEffects: false });
    assert.deepEqual(await p.eval(MOTION), { paused: false, still: false }, '关掉低特效后应恢复');
    // 草图 v4：系统要求少动效时同样停
    await emulateMotion(p, 'reduce');
    await p.waitFor(`${MOTION}.still && ${MOTION}.paused`, 2000, '系统要求少动效时应换静帧');
  },

  async home_start_rejection_is_hint(p, { base }) {
    await p.open(base + page('launch-ok'));
    await waitHome(p);
    // 同一帧连按两次 Enter：第二次被拒（已在启动中），只提示，启动照常
    await p.eval(`for (let i = 0; i < 2; i++) window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))`);
    await p.waitFor(`${TOAST}.includes('已在启动中')`, 2000, '被拒时应提示原因');
    assert.equal((await calls(p, 'startGame')).length, 2);
    assert.ok(!(await p.eval(DIALOG_OPEN)), '提示不应弹框');
    await p.waitFor(`${MODE} === 'launch'`, 2000, '第一次启动应照常进入过场');
  },

  async home_api_rejections_keep_screen(p, { base }) {
    // 未知剧本：getHome / startGame 都被拒，首页照样画出来、不卡住
    await p.open(base + page('nope'));
    assert.deepEqual(await p.eval(`[...document.querySelectorAll('.tmenu button')].map((b) => b.textContent)`),
      ['开始游戏', '图鉴', '深夜与联机', '设置', '存档', '更新']);
    assert.equal((await calls(p, 'uiReady')).length, 1, '首屏画完应通知 uiReady');
    await p.waitFor(`${TOAST}.includes('未知的假剧本')`, 2000, 'getHome 被拒应提示');
    await p.key('Enter');
    await sleep(150);
    assert.equal((await calls(p, 'startGame')).length, 1);
    assert.ok((await p.eval(TOAST)).includes('未知的假剧本'));
    assert.equal(await p.eval(MODE), 'menu', '被拒后留在首页');
    assert.equal(await p.eval(SELECTED), 1);
  },
});
