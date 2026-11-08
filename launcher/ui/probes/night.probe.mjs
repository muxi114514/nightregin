// 深夜与联机、永夜王网格
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { TOAST, answer, calls, clickAt, dialog, shot, waitDialog } from './screen-helpers.mjs';
import {
  ARROW, DESC, LABELS, NIGHT, OFF, ROW, ROWS, SEL, TITLE, VAL, doubleClickAt, openSys, selectRow, setCalls, settleClicks,
} from './sys-helpers.mjs';

const lastSet = async (p) => (await setCalls(p)).at(-1)?.args;
const LORD = (i) => `.lgrid .lord[data-i="${i}"]`;
const GRID_ON = `(!!document.querySelector('.lgrid:not([hidden])'))`;
const LORD_ON = `[...document.querySelectorAll('.lgrid .lord.on')].map((b) => +b.dataset.i)`;
const FOCUS = `(() => { const b = document.querySelector('.lgrid .lord.f'); return b ? +b.dataset.i : -1; })()`;

async function openGrid(p, base, extra = '') {
  await openSys(p, base, NIGHT, extra);
  await selectRow(p, 'lords');
  await sleep(320);
  await p.key('Enter');
  await p.waitFor(GRID_ON, 2000, '永夜王网格没打开');
  await settleClicks();
}

export const tests = {
  async night_rows_and_defaults(p, { base }) {
    await openSys(p, base, NIGHT);
    assert.deepEqual((await p.eval(LABELS)).filter(Boolean), ['强制深夜', '深夜层数', '永夜王', '解锁深夜', '敌人血量', '敌人伤害', '敌人架势']);
    assert.equal(await p.eval(SEL), 'forceDeepNight');
    assert.equal(await p.eval(VAL('deepNightLevel')), '5 层');
    assert.equal(await p.eval(VAL('lords')), '整合包默认');
    assert.equal(await p.eval(VAL('healthScaling')), '120%');
    assert.equal(await p.eval(VAL('damageScaling')), '60%');
    assert.equal(await p.eval(VAL('postureScaling')), '100%');
    await sleep(500);
    await shot(p, 'sys_night');
  },

  async night_level_dim_when_force_off(p, { base }) {
    await openSys(p, base, NIGHT);
    await p.key('ArrowLeft');
    assert.deepEqual(await lastSet(p), { change: { key: 'forceDeepNight', value: false } });
    assert.equal(await p.eval(OFF('deepNightLevel')), true);
    await selectRow(p, 'deepNightLevel');
    const n = (await setCalls(p)).length;
    await p.key('ArrowLeft');
    assert.equal((await setCalls(p)).length, n, '变暗的行 ← → 无效');
  },

  async night_level_stops_at_bounds(p, { base }) {
    await openSys(p, base, NIGHT);
    await selectRow(p, 'deepNightLevel');
    await p.key('ArrowRight');
    assert.equal((await setCalls(p)).length, 0, '5 层已到上限');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('deepNightLevel')), '4 层');
  },

  async night_scaling_step_10(p, { base }) {
    await openSys(p, base, NIGHT);
    await selectRow(p, 'healthScaling');
    await p.key('ArrowRight');
    assert.deepEqual(await lastSet(p), { change: { key: 'healthScaling', value: 130 } });
  },

  async night_scaling_snaps_hand_edited(p, { base }) {
    await openSys(p, base, NIGHT, '&cfg=handEdited');
    assert.equal(await p.eval(VAL('healthScaling')), '125%');
    await selectRow(p, 'healthScaling');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('healthScaling')), '120%');
    await p.key('ArrowDown');
    assert.equal(await p.eval(VAL('damageScaling')), '500%');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('damageScaling')), '300%');
  },

  async night_set_sends_change_shape(p, { base }) {
    await openSys(p, base, NIGHT);
    await selectRow(p, 'deepNightLevel');
    await p.key('ArrowLeft');
    await p.key('ArrowLeft');
    assert.deepEqual(await lastSet(p), { change: { key: 'deepNightLevel', value: 3 } });
  },

  async night_read_failed_row_offers_restore(p, { base }) {
    await openSys(p, base, NIGHT, '&cfg=nighterBad');
    for (const id of ['forceDeepNight', 'deepNightLevel', 'unlockDeepNight']) {
      assert.match(await p.eval(VAL(id)), /读取失败/, id);
    }
    assert.equal(await p.eval(VAL('healthScaling')), '120%');
    assert.equal(await p.eval(VAL('lords')), '整合包默认', 'nighter 坏了不影响永夜王');
    await sleep(320);
    await p.key('Enter');
    const d = await waitDialog(p);
    assert.match(d.msg, /深夜设置.*出厂/);
    assert.equal(d.sel, 1, '默认选取消');
    await answer(p, 'Enter');
    await sleep(100);
    assert.equal((await calls(p, 'restoreConfigDefault')).length, 0);
    await sleep(320);
    await p.key('Enter');
    await waitDialog(p);
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`${VAL('deepNightLevel')} === '5 层'`, 2000, '恢复默认后应刷新');
    assert.deepEqual((await calls(p, 'restoreConfigDefault'))[0].args, { file: 'nighter' });
  },

  async night_game_running_hint(p, { base }) {
    await openSys(p, base, NIGHT, '&cfg=running');
    assert.match(await p.eval(DESC), /重启游戏后生效/);
  },

  async night_dev_mode_shows_keys(p, { base }) {
    await openSys(p, base, NIGHT);
    assert.doesNotMatch(await p.eval(DESC), /nighter\.json/);
    await openSys(p, base, NIGHT, '&dev=1');
    assert.match(await p.eval(DESC), /nighter\.json/);
  },

  async night_external_refresh_updates_value(p, { base }) {
    await openSys(p, base, NIGHT);
    await selectRow(p, 'healthScaling');
    await p.eval(`window.__fakeEmit('game-settings', window.__fakeGameSettings({ healthScaling: { state: 'ok', value: 200 } }))`);
    await sleep(60);
    assert.equal(await p.eval(VAL('healthScaling')), '200%');
    assert.equal(await p.eval(SEL), 'healthScaling');
  },

  async night_write_failed_toast(p, { base }) {
    await openSys(p, base, NIGHT);
    await p.eval(`window.__fakeEmit('config-write-failed', '拒绝访问')`);
    await p.waitFor(`${TOAST}.includes('设置没有写入')`, 1500, '写入失败要提示');
  },

  async lords_row_cycle_restores_picks(p, { base }) {
    await openSys(p, base, NIGHT, '&cfg=picks');
    await selectRow(p, 'lords');
    assert.equal(await p.eval(VAL('lords')), '已挑选 2 位');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('lords')), '无');
    assert.deepEqual(await lastSet(p), { change: { key: 'lords', value: [] } });
    await p.key('ArrowRight');
    assert.equal(await p.eval(VAL('lords')), '已挑选 2 位');
    assert.deepEqual(await lastSet(p), { change: { key: 'lords', value: [1, 3] } });
    await p.key('ArrowRight');
    assert.equal(await p.eval(VAL('lords')), '全部');
    await p.key('ArrowRight');
    assert.equal(await p.eval(VAL('lords')), '整合包默认');
    assert.deepEqual(await lastSet(p), { change: { key: 'lordsFactory' } });
    await p.key('ArrowRight');
    assert.equal(await p.eval(VAL('lords')), '整合包默认', '到头停住');
  },

  async lords_row_skips_picked_without_picks(p, { base }) {
    await openSys(p, base, NIGHT);
    await selectRow(p, 'lords');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('lords')), '全部');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('lords')), '无', '没有已挑选组合时跳过这一档');
  },

  async lords_row_click_opens_grid(p, { base }) {
    await openSys(p, base, NIGHT);
    await clickAt(p, `${ROW('lords')} .lb`);
    await p.waitFor(GRID_ON, 2000, '单击永夜王行应打开网格');
  },

  async night_chance_setter_bad_fails_lords_only(p, { base }) {
    await openSys(p, base, NIGHT, '&cfg=csBad');
    assert.match(await p.eval(VAL('lords')), /读取失败/);
    assert.equal(await p.eval(VAL('deepNightLevel')), '5 层');
    await selectRow(p, 'lords');
    await sleep(320);
    await p.key('Enter');
    const d = await waitDialog(p);
    assert.match(d.msg, /夜王几率设置.*出厂/);
  },

  async lords_custom_odds_label(p, { base }) {
    await openSys(p, base, NIGHT, '&cfg=custom');
    assert.equal(await p.eval(VAL('lords')), '自定义几率');
    await selectRow(p, 'lords');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(VAL('lords')), '全部');
  },

  async lords_factory_grid_starts_empty(p, { base }) {
    await openGrid(p, base);
    assert.deepEqual(await p.eval(LORD_ON), []);
    assert.match(await p.eval(TITLE), /0 \/ 8/);
    await sleep(320);
    await p.key('Enter');
    assert.deepEqual(await lastSet(p), { change: { key: 'lords', value: [0] } });
  },

  async lords_grid_toggle_sends_list(p, { base }) {
    await openGrid(p, base, '&cfg=all');
    assert.deepEqual(await p.eval(LORD_ON), [0, 1, 2, 3, 4, 5, 6, 7]);
    await sleep(320);
    await shot(p, 'sys_lords');
    await p.key('Enter');
    assert.deepEqual(await lastSet(p), { change: { key: 'lords', value: [1, 2, 3, 4, 5, 6, 7] } });
    assert.match(await p.eval(TITLE), /7 \/ 8/);
  },

  async lords_grid_r_all_none(p, { base }) {
    await openGrid(p, base, '&cfg=all');
    await p.key('r');
    assert.deepEqual(await p.eval(LORD_ON), []);
    assert.deepEqual(await lastSet(p), { change: { key: 'lords', value: [] } });
    await p.key('r');
    assert.equal((await p.eval(LORD_ON)).length, 8);
  },

  async lords_grid_navigation(p, { base }) {
    await openGrid(p, base);
    await p.key('ArrowRight');
    await p.key('ArrowRight');
    await p.key('ArrowRight');
    await p.key('ArrowRight');
    assert.equal(await p.eval(FOCUS), 3, '行尾不换行');
    await p.key('ArrowDown');
    assert.equal(await p.eval(FOCUS), 7, '第 8 格是哈尔莫妮亚');
    await p.key('ArrowUp');
    assert.equal(await p.eval(FOCUS), 3);
  },

  async lords_grid_esc_returns_to_row(p, { base }) {
    await openGrid(p, base);
    await p.key('Escape');
    await p.waitFor(`!${GRID_ON}`, 1500, 'Esc 没退出网格');
    assert.equal(await p.eval(SEL), 'lords');
    assert.ok((await p.eval(ROWS)).includes('lords'));
  },

  async lords_grid_enter_guard_300ms(p, { base }) {
    await openSys(p, base, NIGHT);
    await selectRow(p, 'lords');
    await sleep(320);
    await p.key('Enter');
    await p.waitFor(GRID_ON, 2000, '网格没打开');
    await p.key('Enter');
    await sleep(80);
    assert.equal((await setCalls(p)).length, 0, '进网格 300ms 内 Enter 不生效');
  },

  async lords_grid_double_click_toggles_once(p, { base }) {
    await openGrid(p, base, '&cfg=all');
    await doubleClickAt(p, LORD(2));
    await sleep(80);
    assert.equal((await setCalls(p)).length, 1);
    assert.equal(await p.eval(`document.querySelector('${LORD(2)}').classList.contains('on')`), false);
  },

  async lords_grid_arrow_unaffected_by_dialog(p, { base }) {
    await openSys(p, base, NIGHT);
    assert.equal(await dialog(p), null);
    await clickAt(p, ARROW('deepNightLevel', 'l'));
    await sleep(60);
    assert.equal(await p.eval(VAL('deepNightLevel')), '4 层');
  },
};
