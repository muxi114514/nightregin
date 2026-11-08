// 图鉴：和永夜王互相跳转、鼠标
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { clickAt, page, waitHome } from './screen-helpers.mjs';
import { MENU_ITEM, NIGHT, doubleClickAt, openSys, selectRow, settleClicks } from './sys-helpers.mjs';
import { FILTER, FOCUS, HINTS, NAME, ON, TAB, ZONE, gotoCell, gotoTab, openWiki } from './wiki-helpers.mjs';

const CELL = (i) => `#wbox .wcell[data-i="${i}"]`;
const LORD_FOCUS = `(() => { const b = document.querySelector('.lgrid .lord.f'); return b ? +b.dataset.i : -1; })()`;
const IN_LORDS = `(!!document.querySelector('#sys.on.lords'))`;
const IN_NIGHT = `(!!document.querySelector('#sys.on:not(.lords)'))`;

/** 打开永夜王网格，走到第 i 位（每行 4 格） */
async function openLordsAt(p, base, i) {
  await openSys(p, base, NIGHT, '&cfg=picks');
  await selectRow(p, 'lords');
  await p.key('Enter');
  await p.waitFor(IN_LORDS, 2000, '永夜王网格没有打开');
  await sleep(320);
  if (i >= 4) await p.key('ArrowDown');
  const col = i >= 4 ? i - 4 : i;
  for (let n = 0; n < col; n++) await p.key('ArrowRight');
  assert.equal(await p.eval(LORD_FOCUS), i);
}

async function enemy(p, base, cell) {
  await openWiki(p, base);
  await gotoTab(p, '敌人');
  await gotoCell(p, cell);
}

export const tests = {
  async lords_f_jumps(p, { base }) {
    await openLordsAt(p, base, 6);
    await p.key('f');
    await p.waitFor(ON, 2000, 'F 没有跳到图鉴');
    await p.waitFor(`${NAME} === '卡莉果'`, 2000, '没有选中卡莉果');
    assert.equal(await p.eval(TAB), '敌人');
  },

  async lords_f_esc_returns_focus(p, { base }) {
    await openLordsAt(p, base, 6);
    await p.key('f');
    await p.waitFor(`${NAME} === '卡莉果'`, 2000, 'F 没有跳到图鉴');
    await sleep(560);
    await p.key('Escape');
    await p.waitFor(IN_LORDS, 2000, 'Esc 应回到永夜王网格');
    await p.waitFor(`${LORD_FOCUS} === 6`, 2000, '焦点应还在第 7 位');
    assert.equal(await p.eval(ON), false);
  },

  async harmonia_maps_to_row_8(p, { base }) {
    await openLordsAt(p, base, 7);
    await p.key('f');
    await p.waitFor(`${NAME} === '哈尔莫妮亚'`, 2000, '第 8 格应对应 NightBossMenuParam 第 8 行');
  },

  async enter_opens_lords(p, { base }) {
    await enemy(p, base, 2);
    assert.match(await p.eval(HINTS), /永夜王设置/);
    await p.key('Enter');
    await p.waitFor(IN_LORDS, 2000, 'Enter 应进永夜王网格');
    await p.waitFor(`${LORD_FOCUS} === 2`, 2000, '焦点应在第 3 位');
    assert.equal(await p.eval(ON), false);
  },

  async no_lord_enter_does_nothing(p, { base }) {
    await enemy(p, base, 7);
    assert.doesNotMatch(await p.eval(HINTS), /永夜王设置/);
    await p.key('Enter');
    await sleep(200);
    assert.equal(await p.eval(ON), true);
    assert.equal(await p.eval(NAME), '黑夜王');
  },

  async lords_back_and_forth_no_stack(p, { base }) {
    await openLordsAt(p, base, 3);
    await p.key('f');
    await p.waitFor(`${NAME} === '玛利斯'`, 2000, 'F 没有跳到图鉴');
    await sleep(560);
    await p.key('Enter');
    await p.waitFor(IN_LORDS, 2000, '从永夜王跳来时 Enter 应回去');
    await sleep(560);
    assert.equal(await p.eval(LORD_FOCUS), 3);
    await p.key('f');
    await p.waitFor(ON, 2000, '第二次 F 没有跳到图鉴');
    await sleep(560);
    await p.key('Escape');
    await p.waitFor(IN_LORDS, 2000, '一次 Esc 应回永夜王');
    await sleep(560);
    await p.key('Escape');
    await p.waitFor(IN_NIGHT, 2000, '再按 Esc 应回深夜与联机');
    assert.equal(await p.eval(ON), false);
  },

  async still_mouse_does_not_select(p, { base }) {
    await openWiki(p, base);
    const c = await p.center(CELL(2));
    await p.mouseMove(c.x, c.y);
    await p.waitFor(`${FOCUS} === 2`, 2000, '鼠标移过去没有选中');
    await p.key('ArrowDown');
    assert.equal(await p.eval(FOCUS), 7);
    await p.key('e');
    await p.key('q');
    await sleep(200);
    assert.equal(await p.eval(FOCUS), 7, '重绘后静止的鼠标不应抢选中项');
  },

  async hover_selects(p, { base }) {
    await openWiki(p, base);
    const c = await p.center(CELL(2));
    await p.mouseMove(c.x, c.y);
    await p.waitFor(`${FOCUS} === 2`, 2000, '悬停没有选中');
    assert.equal(await p.eval(NAME), '测试武器3');
    assert.equal(await p.eval(ZONE), 2);
  },

  async click_selects_not_activate(p, { base }) {
    await enemy(p, base, 0);
    await clickAt(p, CELL(2));
    await p.waitFor(`${FOCUS} === 2`, 2000, '单击没有选中');
    await sleep(200);
    assert.equal(await p.eval(ON), true, '单击不进永夜王');
  },

  async double_click_no_passthrough(p, { base }) {
    await enemy(p, base, 0);
    await doubleClickAt(p, CELL(3));
    await sleep(200);
    assert.equal(await p.eval(ON), true);
    assert.equal(await p.eval(FOCUS), 3);
  },

  async click_guard_after_open(p, { base }) {
    await p.open(base + page('home-ok'));
    await waitHome(p);
    await settleClicks();
    await clickAt(p, MENU_ITEM(2));
    await p.waitFor(`!!document.querySelector(${JSON.stringify(CELL(3))})`, 2000, '条目没有画出来');
    const c = await p.center(CELL(3));
    await p.click(c.x, c.y);
    await sleep(100);
    assert.equal(await p.eval(FOCUS), 0, '进页面 500ms 内的点击要丢弃');
  },

  async filter_arrow_click(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, `#wrows .row[data-z="0"] .ar.r`);
    await p.waitFor(`${FILTER} === '本版新增'`, 2000, '单击箭头没有改筛选');
    assert.equal(await p.eval(ZONE), 0);
  },

  async filter_row_click_only_selects(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, `#wrows .row[data-z="0"] .lb`);
    await p.waitFor(`${ZONE} === 0`, 2000, '单击显示行应选中它');
    assert.equal(await p.eval(FILTER), '全部');
  },

  async search_row_click_edits(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, `#wrows .row[data-z="1"]`);
    await p.waitFor(`document.activeElement?.id === 'wq'`, 2000, '单击查找行应开始输入');
  },

  async hints_clickable(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, `#ckeys [data-k="Escape"]`);
    await p.waitFor(`!${ON}`, 2000, '点「返回」应回首页');
  },

  async enter_after_click_does_not_reclick(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, CELL(1));
    await p.waitFor(`${FOCUS} === 1`, 2000, '单击没有选中');
    await p.key('ArrowRight');
    await p.key('ArrowRight');
    assert.equal(await p.eval(FOCUS), 3);
    await p.key('Enter');
    await sleep(150);
    assert.equal(await p.eval(FOCUS), 3, 'Enter 不应点回鼠标点过的格子');
    await p.key(' ');
    await sleep(150);
    assert.equal(await p.eval(FOCUS), 3, '空格也不应点回去');
  },

  async enter_after_tab_click_keeps_tab(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, '#codex .tab:nth-child(2)');
    await p.waitFor(`${TAB} === '护符'`, 2000, '单击标签没有切换');
    await p.key('q');
    await p.waitFor(`${TAB} === '武器'`, 2000, 'Q 没有切换');
    await p.key(' ');
    await sleep(150);
    assert.equal(await p.eval(TAB), '武器', '空格不应点回鼠标点过的标签');
  },

  async wheel_then_click_keeps_scroll(p, { base }) {
    await openWiki(p, base, '&wiki=large');
    const c = await p.center('#wbox');
    await p.mouseMove(c.x, c.y);
    await sleep(100);
    await p.wheel(c.x, c.y, 2400);
    await p.waitFor(`document.querySelector('#wbox').scrollTop > 1000`, 2000, '滚轮没有滚动');
    await sleep(150);
    const top = await p.eval(`document.querySelector('#wbox').scrollTop`);
    const target = await p.eval(`+document.elementFromPoint(${c.x}, ${c.y}).closest('.wcell').dataset.i`);
    await p.click(c.x, c.y);
    await sleep(150);
    assert.equal(await p.eval(FOCUS), target);
    assert.equal(await p.eval(`document.querySelector('#wbox').scrollTop`), top, '点击不应滚回旧选中项');
  },

  async empty_category_returns_focus_to_list(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '法术');
    assert.equal(await p.eval(ZONE), 1, '空分类只能在查找行');
    await p.key('e');
    await p.waitFor(`${TAB} === '大地图'`, 2000, '没有切到大地图');
    assert.equal(await p.eval(ZONE), 2, '离开空分类后回到条目区');
  },
};
