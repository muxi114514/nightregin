// 设置与关于
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { TOAST, calls, clickAt, shot } from './screen-helpers.mjs';
import { DESC, LABELS, OPTIONS, ROW, SEL, VAL, doubleClickAt, openSys, selectRow } from './sys-helpers.mjs';

const lastSet = async (p) => (await calls(p, 'setOption')).at(-1)?.args;
const PAUSED = `document.documentElement.classList.contains('paused')`;

async function pressAbout(p, n, gap = 80) {
  for (let i = 0; i < n; i++) {
    await p.key('Enter');
    await sleep(gap);
  }
}

export const tests = {
  async options_rows_order(p, { base }) {
    await openSys(p, base, OPTIONS);
    assert.deepEqual(await p.eval(LABELS), ['启动前备份存档', '游戏启动后', '低特效', '整理问题记录', '游戏所在文件夹', '建桌面快捷方式', '关于']);
    assert.equal(await p.eval(SEL), 'backupBeforeStart');
    await sleep(500);
    await shot(p, 'sys_options');
  },

  async options_after_launch_stops_at_ends(p, { base }) {
    await openSys(p, base, OPTIONS);
    await selectRow(p, 'afterLaunch');
    assert.equal(await p.eval(VAL('afterLaunch')), '隐入托盘');
    await p.key('ArrowLeft');
    assert.equal((await calls(p, 'setOption')).length, 0, '第一档按 ← 不动');
    await p.key('ArrowRight');
    assert.deepEqual(await lastSet(p), { change: { key: 'afterLaunch', value: 'keep' } });
    await p.key('ArrowRight');
    assert.deepEqual(await lastSet(p), { change: { key: 'afterLaunch', value: 'close' } });
    await p.key('ArrowRight');
    assert.equal((await calls(p, 'setOption')).length, 2, '最后一档按 → 不动');
  },

  async options_low_effects_applies_now(p, { base }) {
    await openSys(p, base, OPTIONS, '&motion=1');
    await p.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
    await sleep(60);
    assert.equal(await p.eval(PAUSED), false);
    await selectRow(p, 'lowEffects');
    await p.key('ArrowRight');
    await p.waitFor(PAUSED, 1500, '低特效应立刻生效');
    assert.deepEqual(await lastSet(p), { change: { key: 'lowEffects', value: true } });
  },

  async options_choose_dir_cancel_is_silent(p, { base }) {
    await openSys(p, base, OPTIONS, '&dircancel=1');
    await selectRow(p, 'gameDir');
    await sleep(320);
    await p.key('Enter');
    await sleep(150);
    assert.equal((await calls(p, 'chooseGameDir')).length, 1);
    assert.equal(await p.eval(TOAST), '', '取消不提示');
  },

  async options_choose_dir_rejected_toast(p, { base }) {
    await openSys(p, base, OPTIONS, '&nodir=1');
    await selectRow(p, 'gameDir');
    await sleep(320);
    await p.key('Enter');
    await p.waitFor(`${TOAST}.includes('没有游戏')`, 1500, '选错要提示');
  },

  async options_shortcut_toast_path(p, { base }) {
    await openSys(p, base, OPTIONS);
    await selectRow(p, 'shortcut');
    await sleep(320);
    await p.key('Enter');
    await p.waitFor(`${TOAST}.includes('桌面')`, 1500, '建好要提示');
  },

  async about_five_presses_enable_dev(p, { base }) {
    await openSys(p, base, OPTIONS);
    await selectRow(p, 'about');
    await sleep(320);
    await pressAbout(p, 5);
    await p.waitFor(`${TOAST}.includes('已打开开发模式')`, 1500, '没打开开发模式');
    assert.deepEqual(await lastSet(p), { change: { key: 'devMode', value: true } });
  },

  async about_five_clicks_enable_dev(p, { base }) {
    await openSys(p, base, OPTIONS);
    await doubleClickAt(p, `${ROW('about')} .lb`);
    await doubleClickAt(p, `${ROW('about')} .lb`);
    await clickAt(p, `${ROW('about')} .lb`);
    await p.waitFor(`${TOAST}.includes('已打开开发模式')`, 1500, '快速点击 5 次没打开开发模式');
  },

  async about_slow_presses_do_not_count(p, { base }) {
    await openSys(p, base, OPTIONS);
    await selectRow(p, 'about');
    await sleep(320);
    await pressAbout(p, 4, 900);
    await sleep(100);
    assert.equal((await calls(p, 'setOption')).length, 0, '3 秒内连按才算');
  },

  async about_shows_versions_and_credit(p, { base }) {
    await openSys(p, base, OPTIONS);
    await selectRow(p, 'about');
    await p.waitFor(`${DESC}.includes('1.9.5')`, 1500, '关于面板没写版本');
    const d = await p.eval(DESC);
    for (const want of ['0.1.0', '0.10.1', '1.3.3.0', '未配置', 'Nightreign Golden Age by max-z', 'launcher\\licenses']) {
      assert.ok(d.includes(want), want);
    }
  },
};
