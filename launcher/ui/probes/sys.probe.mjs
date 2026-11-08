// 次级页骨架：导航跑真实首页；取值 / 动作 / 失败 / 分隔 / 刷新跑 probe.html 的演示行
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { calls, clickAt, waitHome, page } from './screen-helpers.mjs';
import {
  ARROW, MENU_ITEM, MENU_SEL, NIGHT, ROW, SAVES, SEL, SYS_ON, TAB, VAL, doubleClickAt, openSys, selectRow, settleClicks,
} from './sys-helpers.mjs';

const DEMO = 'probe.html?fake=home-ok&rows=1';
const STATE = (k) => `window.__rowsState.${k}`;

async function demo(p, base) {
  await p.open(base + DEMO);
  await p.waitFor(`document.querySelectorAll('.olist .row').length > 0`, 3000, '演示行没有画出来');
  await settleClicks();
}

export const tests = {
  async sys_open_from_home_default_row(p, { base }) {
    await openSys(p, base, NIGHT);
    assert.equal(await p.eval(TAB), '深夜与联机');
    const first = await p.eval(`document.querySelector('.olist .row:not(.hd)').dataset.id`);
    assert.equal(await p.eval(SEL), first);
  },

  async sys_q_e_switch_tabs_keep_row(p, { base }) {
    await openSys(p, base, NIGHT);
    await p.key('ArrowDown');
    const row = await p.eval(SEL);
    await p.key('e');
    await p.waitFor(`${TAB.replace(/;$/, '')} === '设置'`, 2000, 'E 没切到设置');
    await p.key('q');
    await p.waitFor(`${TAB} === '深夜与联机'`, 2000, 'Q 没切回来');
    await p.waitFor(`${SEL} === ${JSON.stringify(row)}`, 2000, '切回来光标没回到原来的行');
    await p.key('q');
    await p.waitFor(`${TAB} === '存档'`, 2000, '第一页按 Q 应回绕到最后一页');
  },

  async sys_value_click_only_selects(p, { base }) {
    await demo(p, base);
    await clickAt(p, `${ROW('n')} .lb`);
    await sleep(100);
    assert.equal(await p.eval(SEL), 'n');
    assert.equal(await p.eval(STATE('n')), 50, '单击取值行不应改值');
  },

  async sys_inline_arrow_changes(p, { base }) {
    await demo(p, base);
    await clickAt(p, ARROW('n', 'r'));
    await sleep(60);
    assert.equal(await p.eval(STATE('n')), 60);
    assert.equal(await p.eval(VAL('n')), '60%');
    for (let i = 0; i < 6; i++) {
      await clickAt(p, ARROW('n', 'l'));
      await sleep(30);
    }
    assert.equal(await p.eval(STATE('n')), 10, '到边界应停住，不回绕');
  },

  async sys_action_click_executes_once(p, { base }) {
    await demo(p, base);
    await clickAt(p, `${ROW('a')} .lb`);
    await sleep(80);
    assert.equal(await p.eval(STATE('runs')), 1);
    await sleep(350);
    await doubleClickAt(p, `${ROW('a')} .lb`);
    await sleep(80);
    assert.equal(await p.eval(STATE('runs')), 2, '双击只执行一次');
  },

  async sys_raw_click_counts_every_click(p, { base }) {
    await demo(p, base);
    await doubleClickAt(p, `${ROW('raw')} .lb`);
    await sleep(80);
    assert.equal(await p.eval(STATE('raw')), 2, 'rawClicks 的行双击算两下');
  },

  async sys_failed_row_enter_offers_restore(p, { base }) {
    await demo(p, base);
    await selectRow(p, 'f');
    assert.match(await p.eval(VAL('f')), /读取失败/);
    await p.key('ArrowRight');
    assert.equal(await p.eval(STATE('fSet')), 0, '读取失败的行 ← → 无效');
    await sleep(320);
    await p.key('Enter');
    assert.equal(await p.eval(STATE('failed')), 1, 'Enter 触发恢复默认');
  },

  async sys_off_row_ignores_arrows(p, { base }) {
    await demo(p, base);
    await selectRow(p, 'off');
    await p.key('ArrowRight');
    assert.equal(await p.eval(STATE('off')), 30);
  },

  async sys_separator_skipped(p, { base }) {
    await demo(p, base);
    await selectRow(p, 'a');
    await p.key('ArrowDown');
    assert.equal(await p.eval(SEL), 'raw');
    await p.key('ArrowDown');
    assert.equal(await p.eval(SEL), 'f', '↓ 应跳过分隔行');
  },

  async sys_esc_and_right_click_back(p, { base }) {
    await openSys(p, base, SAVES);
    await p.key('Escape');
    await p.waitFor(`!${SYS_ON}`, 2000, 'Esc 没返回');
    assert.equal(await p.eval(MENU_SEL), SAVES, '回到来时那一项');
    await settleClicks();
    await clickAt(p, MENU_ITEM(NIGHT));
    await p.waitFor(SYS_ON, 2000, '没进页面');
    await settleClicks();
    await p.click(300, 400, { button: 'right' });
    await p.waitFor(`!${SYS_ON}`, 2000, '右键没返回');
    assert.equal(await p.eval(MENU_SEL), NIGHT);
  },

  async sys_enter_never_starts_game(p, { base }) {
    await openSys(p, base, NIGHT);
    await sleep(320);
    await p.key('Enter');
    await sleep(150);
    assert.equal((await calls(p, 'startGame')).length, 0);
  },

  async sys_hover_needs_real_move(p, { base }) {
    await demo(p, base);
    const c = await p.center(`${ROW('p')} .lb`);
    await p.mouseMove(c.x, c.y);
    await sleep(80);
    assert.equal(await p.eval(SEL), 'p', '真移动才选中');
    await p.key('ArrowUp');
    const moved = await p.eval(SEL);
    await p.eval(`document.querySelector('.olist').dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: ${c.x}, clientY: ${c.y} }))`);
    await sleep(80);
    assert.equal(await p.eval(SEL), moved, '鼠标没动时不抢选中');
  },

  async sys_click_guard_after_page_switch(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await settleClicks();
    await clickAt(p, MENU_ITEM(NIGHT));
    await p.waitFor(SYS_ON, 2000, '没进页面');
    // 500ms 内的点击丢弃：内联箭头不生效
    await clickAt(p, ARROW('forceDeepNight', 'l'));
    await sleep(80);
    assert.equal((await calls(p, 'setGameSetting')).length, 0);
  },

  async sys_background_refresh_keeps_cursor(p, { base }) {
    await demo(p, base);
    await selectRow(p, 'a9');
    const top = await p.eval(`document.querySelector('.olist').scrollTop`);
    assert.ok(top > 0, '演示行要够长，才能滚动');
    await p.eval(`window.__rowsRefresh()`);
    await sleep(60);
    assert.equal(await p.eval(SEL), 'a9');
    assert.equal(await p.eval(`document.querySelector('.olist').scrollTop`), top);
  },
};
