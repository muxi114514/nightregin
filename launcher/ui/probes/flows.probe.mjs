// 问题记录流程、失败横带动作、首页状态行、首次运行、页面重载
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { BANNER, MODE, TOAST, answer, calls, clickAt, dialog, page, waitDialog, waitHome } from './screen-helpers.mjs';
import { OPTIONS, SEL, SYS_ON, TAB, doubleClickAt, openSys, selectRow, settleClicks } from './sys-helpers.mjs';

const STATUS_BTN = (i) => `#home .status > :nth-child(${i})`;
const homeState = (o) => JSON.stringify({ version: '1.9.5', deepNight: { enabled: true, level: 5 }, issues: [], lowEffects: false, devMode: false, ...o });
const issue = (kind, detail = '', blocking = false) => ({ kind, blocking, detail });
const emitHome = (p, o) => p.eval(`window.__fakeEmit('home-state', ${homeState(o)})`);

async function openReport(p, base, extra = '') {
  await openSys(p, base, OPTIONS, extra);
  await selectRow(p, 'report');
  await sleep(320);
  await p.key('Enter');
}

/** 首页打开并过点击防抖 */
async function home(p, base, script = 'home-ok', extra = '') {
  await p.open(base + page(script, extra));
  await waitHome(p);
  await settleClicks();
}

/** 把启动流程跑到失败横带 */
async function failure(p, base, kind, extra = '') {
  await home(p, base, kind === 'chain' ? 'launch-chain' : `launch-fail-${kind}`, extra);
  await p.key('Enter');
  return waitDialog(p, 8000, '失败横带没有出现');
}

export const tests = {
  async report_lists_dumps_defaults_cancel(p, { base }) {
    await openReport(p, base, '&dumps=1');
    const d = await waitDialog(p);
    assert.match(d.msg + d.sub, /崩溃转储可能包含账号名等信息/);
    const text = await p.eval(`document.querySelector('.dlg.on').textContent`);
    assert.ok(text.includes('a.dmp') && text.includes('MB'), text);
    assert.equal(d.sel, 1, '默认选取消');
    await answer(p, 'Enter');
    await sleep(120);
    assert.equal((await calls(p, 'createReport')).length, 0);
  },

  async report_generates_and_reveals(p, { base }) {
    await openReport(p, base, '&dumps=1');
    await waitDialog(p);
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`${TOAST}.includes('问题记录已放到桌面')`, 2000, '没有成功提示');
    assert.equal((await calls(p, 'revealPath')).length, 1);
  },

  async report_without_dumps_skips_dialog(p, { base }) {
    await openReport(p, base);
    await p.waitFor(`(window.__calls || []).some((c) => c.name === 'createReport')`, 1500, '没有转储时直接生成');
    assert.equal(await dialog(p), null);
  },

  async report_reentry_ignored(p, { base }) {
    await openReport(p, base, '&reportslow=1');
    await sleep(350);
    await p.key('Enter');
    await sleep(120);
    assert.equal((await calls(p, 'createReport')).length, 1);
  },

  async report_from_tray_event(p, { base }) {
    await home(p, base);
    await p.eval(`window.__fakeEmit('open-report')`);
    await p.waitFor(`(window.__calls || []).some((c) => c.name === 'createReport')`, 1500, '托盘发起的没执行');
    assert.equal((await calls(p, 'ackOpenReport')).length, 1, '收到事件要回执，Rust 才知道界面还活着');
  },

  async report_failure_toast(p, { base }) {
    await openReport(p, base, '&reportfail=1');
    await p.waitFor(`${TOAST}.includes('磁盘已满')`, 1500, '失败要提示原因');
  },

  async failure_rows_follow_table(p, { base }) {
    const cases = [
      ['gameNotFound', ['重新指定游戏所在文件夹', '整理问题记录', '返回'], 0],
      ['me3FilesMissing', ['整理问题记录', '返回'], 1],
      ['suffixMismatch', ['一键对齐后继续', '返回'], 0],
      ['configWrite', ['重试', '整理问题记录', '返回'], 0],
      ['spawnFailed', ['整理问题记录', '重试', '返回'], 1],
      ['me3Exit', ['整理问题记录', '重试', '返回'], 1],
      ['gameNeverStarted', ['整理问题记录', '重试', '返回'], 1],
      ['gameExitedEarly', ['整理问题记录', '重试', '返回'], 1],
    ];
    for (const [kind, opts, sel] of cases) {
      const d = await failure(p, base, kind);
      assert.deepEqual(d.opts, opts, kind);
      assert.equal(d.sel, sel, kind);
    }
  },

  async home_banner_fix_actions(p, { base }) {
    await home(p, base);
    for (const [kind, call, text] of [
      ['chanceSetterSuffix', 'alignSuffix', '一键改正'],
      ['everdarkMismatch', 'fixEverdark', '一键改正'],
      ['lastExitAbnormal', 'createReport', '整理问题记录'],
    ]) {
      await emitHome(p, { issues: [issue(kind, 'x')] });
      await sleep(60);
      assert.equal((await p.eval(BANNER)).action, text, kind);
      await settleClicks();
      await clickAt(p, '#home .issue .opt');
      await p.waitFor(`(window.__calls || []).some((c) => c.name === '${call}')`, 1500, `${kind} 没调 ${call}`);
    }
  },

  async home_save_busy_has_own_text(p, { base }) {
    await home(p, base);
    await emitHome(p, { issues: [issue('saveBusy')] });
    await sleep(60);
    const b = await p.eval(BANNER);
    assert.match(b.msg, /存档正被其他程序占用/);
    assert.doesNotMatch(b.msg, /备份失败/);
    assert.ok(!b.action, '首页这一条不开始游戏');
  },

  async failure_align_and_retry_keeps_job(p, { base }) {
    await failure(p, base, 'suffixMismatch');
    await answer(p, 'Enter');
    await p.waitFor(`(window.__calls || []).filter((c) => c.name === 'startGame').length === 2`, 3000, '对齐后没有重新开始');
    assert.equal((await calls(p, 'alignSuffix')).length, 1);
    assert.deepEqual((await calls(p, 'startGame'))[1].args, { skipBackup: false, ignoreVersion: false });
  },

  async chained_choose_dir_keeps_ignore_version(p, { base }) {
    await failure(p, base, 'gameVersionMismatch', '&dev=1');
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`(window.__calls || []).filter((c) => c.name === 'startGame').length === 2`, 3000, '仍然启动没有发出');
    assert.deepEqual((await calls(p, 'startGame'))[1].args, { skipBackup: false, ignoreVersion: true });
  },

  async chained_retry_keeps_ignore_version(p, { base }) {
    const d = await failure(p, base, 'chain', '&dev=1');
    assert.equal(d.opts[0], '仍然启动');
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`(window.__calls || []).filter((c) => c.name === 'startGame').length === 2`, 3000, '仍然启动没有发出');
    const next = await waitDialog(p, 6000, '第二次失败横带没有出现');
    assert.equal(next.opts[next.sel], '重试');
    await answer(p, 'Enter');
    await p.waitFor(`(window.__calls || []).filter((c) => c.name === 'startGame').length === 3`, 3000, '重试没有发出');
    assert.deepEqual((await calls(p, 'startGame'))[2].args, { skipBackup: false, ignoreVersion: true });
  },

  async home_status_segments_navigate(p, { base }) {
    await home(p, base);
    await clickAt(p, STATUS_BTN(2));
    await p.waitFor(SYS_ON, 1500, '深夜段没跳转');
    assert.equal(await p.eval(TAB), '深夜与联机');

    const cases = [
      [{ issues: [] }, '设置', null],
      [{ issues: [issue('gameNotFound')] }, '设置', 'gameDir'],
      [{ issues: [issue('me3FilesMissing', 'x')] }, '设置', 'about'],
      [{ issues: [issue('saveBusy')] }, '存档', 'suffix'],
      [{ issues: [issue('chanceSetterSuffix', 'x')] }, '存档', 'suffix'],
      [{ issues: [issue('everdarkMismatch')] }, '深夜与联机', 'lords'],
      [{ issues: [issue('lastExitAbnormal')] }, '设置', 'report'],
    ];
    for (const [state, tab, row] of cases) {
      await home(p, base);
      await emitHome(p, state);
      await sleep(60);
      await clickAt(p, STATUS_BTN(3));
      await p.waitFor(SYS_ON, 1500, `${tab} 没跳转`);
      assert.equal(await p.eval(TAB), tab);
      if (row) await p.waitFor(`${SEL} === '${row}'`, 1500, `${tab} 光标应在 ${row}`);
    }
  },

  async home_status_double_click_once(p, { base }) {
    await home(p, base);
    await doubleClickAt(p, STATUS_BTN(2));
    await p.waitFor(SYS_ON, 1500, '没跳转');
    await sleep(100);
    assert.equal(await p.eval(TAB), '深夜与联机');
  },

  async firstrun_full_flow(p, { base }) {
    await p.open(base + page('home-ok', '&first=full'));
    let d = await waitDialog(p);
    assert.match(d.msg, /账号/);
    await answer(p, 'Enter');
    await sleep(100);
    d = await waitDialog(p);
    assert.match(d.msg, /原版存档/);
    await answer(p, 'Enter');
    await sleep(100);
    d = await waitDialog(p);
    assert.match(d.sub + d.msg, /不一致/);
    await answer(p, 'Enter');
    await sleep(100);
    d = await waitDialog(p);
    assert.match(d.msg, /快捷方式/);
    await answer(p, 'Enter');
    await p.waitFor(`(window.__calls || []).some((c) => c.name === 'finishFirstRun')`, 1500, '没有写完成');
    assert.deepEqual((await calls(p, 'finishFirstRun'))[0].args, { account: '76561198000000001' });
    assert.equal((await calls(p, 'copyFromVanilla')).length, 1);
    assert.equal((await calls(p, 'alignSuffix')).length, 1);
    assert.equal((await calls(p, 'createShortcut')).length, 1);
  },

  async firstrun_game_missing_cannot_skip(p, { base }) {
    await p.open(base + page('home-ok', '&first=missing&nodir=1'));
    const d = await waitDialog(p);
    assert.deepEqual(d.opts, ['选择文件夹']);
    await sleep(350);
    await p.key('Escape');
    await sleep(120);
    assert.ok(await dialog(p), 'Esc 不能跳过');
    await p.key('Enter');
    await p.waitFor(`${TOAST}.includes('没有游戏')`, 1500, '选错要提示');
    assert.ok(await dialog(p), '选错后还留在这一步');
  },

  async firstrun_skip_all_optional_steps(p, { base }) {
    await p.open(base + page('home-ok', '&first=full'));
    for (const skip of [3, 1, 1, 1]) {
      const d = await waitDialog(p);
      await sleep(350);
      for (let i = 0; i < skip; i++) await p.key('ArrowRight');
      await p.key('Enter');
      await sleep(120);
      assert.ok(d.opts.length >= 2);
    }
    await p.waitFor(`(window.__calls || []).some((c) => c.name === 'finishFirstRun')`, 1500, '没有写完成');
    assert.deepEqual((await calls(p, 'finishFirstRun'))[0].args, {});
    assert.equal((await calls(p, 'copyFromVanilla')).length, 0);
  },

  async firstrun_call_failure_continues(p, { base }) {
    await p.open(base + page('home-ok', '&first=copyfail'));
    await waitDialog(p);
    await answer(p, 'Enter');
    await p.waitFor(`${TOAST}.includes('复制失败')`, 1500, '失败要提示');
    const d = await waitDialog(p);
    assert.match(d.msg, /快捷方式/);
  },

  async firstrun_not_shown_when_done(p, { base }) {
    await home(p, base);
    assert.equal(await dialog(p), null);
    assert.equal((await calls(p, 'finishFirstRun')).length, 0);
  },

  async launch_view_leaves_sys_page(p, { base }) {
    await openSys(p, base, OPTIONS);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'backup' })`);
    await p.waitFor(`!${SYS_ON} && ${MODE} === 'launch'`, 2000, '托盘开始游戏时要离开次级页');
  },

  async reload_resyncs_launch_view(p, { base }) {
    await p.open(base + page('home-ok', '&view=waking'));
    await p.waitFor(`${MODE} === 'launch'`, 2000, '重载后没有接回启动过场');
  },
};
