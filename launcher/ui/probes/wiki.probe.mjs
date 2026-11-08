// 图鉴：页面壳、分类标签、条目区、筛选与查找
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { clickAt, page, waitHome } from './screen-helpers.mjs';
import { MENU_SEL } from './sys-helpers.mjs';
import {
  COUNT, FILTER, FOCUS, NAME, NONE, ON, QUERY, TAB, ZONE, gotoCell, gotoTab, openWiki, startSearch, typeText,
} from './wiki-helpers.mjs';

const TAB_CELL = (n) => `#codex .tab:nth-child(${n})`;
const has = (sel) => `(!!document.querySelector(${JSON.stringify(sel)}))`;
const NOT_EDITING = `document.activeElement?.id !== 'wq'`;

export const tests = {
  async opens_from_home(p, { base }) {
    await openWiki(p, base);
    assert.equal(await p.eval(ON), true);
    assert.equal(await p.eval(TAB), '武器');
    assert.equal(await p.eval(NAME), '测试武器1');
  },

  async esc_returns_home(p, { base }) {
    await openWiki(p, base);
    await p.key('Escape');
    await p.waitFor(`!${ON}`, 2000, 'Esc 没有离开图鉴');
    assert.equal(await p.eval(MENU_SEL), 2);
  },

  async qe_switch_and_wrap(p, { base }) {
    await openWiki(p, base);
    await p.key('q');
    await p.waitFor(`${TAB} === '机制'`, 2000, '第一个分类按 Q 应回绕到最后');
    await p.key('e');
    await p.waitFor(`${TAB} === '武器'`, 2000, 'E 应回绕到第一个');
    const seen = [];
    for (let i = 0; i < 6; i++) {
      await p.key('e');
      seen.push(await p.eval(TAB));
    }
    assert.deepEqual(seen, ['护符', '法术', '大地图', '敌人', '机制', '武器']);
  },

  async tab_click(p, { base }) {
    await openWiki(p, base);
    await clickAt(p, TAB_CELL(2));
    await p.waitFor(`${TAB} === '护符'`, 2000, '单击标签没有切换');
  },

  async unknown_layout_is_list(p, { base }) {
    await openWiki(p, base, '&wiki=odd');
    assert.equal(await p.eval(has('#wbox .row[data-i]')), true);
    assert.equal(await p.eval(has('#wbox .wcell[data-i]')), false);
  },

  async new_badge_only_from_ids(p, { base }) {
    await openWiki(p, base);
    assert.equal(await p.eval(has(`${TAB_CELL(1)} .nb`)), true);
    assert.equal(await p.eval(has(`${TAB_CELL(2)} .nb`)), false);
  },

  async tabs_scroll_follow(p, { base }) {
    await openWiki(p, base, '&wiki=manycats');
    for (let i = 0; i < 13; i++) await p.key('e');
    await p.waitFor(`${TAB} === '分类14'`, 2000, '没有翻到最后一个分类');
    await p.waitFor(`(() => {
      const t = document.querySelector('#codex .tab.on').getBoundingClientRect();
      const b = document.querySelector('#codex .tabs').getBoundingClientRect();
      return t.left >= b.left && t.right <= b.right;
    })()`, 2000, '选中的标签跑出了横幅');
  },

  async seven_tabs_all_fit(p, { base }) {
    await openWiki(p, base);
    assert.equal(await p.eval(`document.querySelectorAll('#codex .tab').length`), 7);
    const fit = await p.eval(`(() => {
      const b = document.querySelector('#codex .tabview').getBoundingClientRect();
      return [...document.querySelectorAll('#codex .tab')].every((t) => {
        const r = t.getBoundingClientRect();
        return r.width > 0 && r.left >= b.left && r.right <= b.right + 0.5;
      });
    })()`);
    assert.equal(fit, true, '7 个分类应该都在横幅里');
    assert.equal(await p.eval(has('#codex .tabs.many')), false);
  },

  async malformed_fields_render(p, { base }) {
    await openWiki(p, base, '&wiki=malformed');
    assert.equal(await p.eval(NAME), '缺值');
    assert.equal(await p.eval(COUNT), 1, '没有名字和不是对象的条目要跳过');
    assert.equal(await p.eval(`document.querySelectorAll('#codex .tab').length`), 2, '没有名字和不是对象的分类要跳过');
    const panel = await p.eval(`document.querySelector('#wdesc').textContent`);
    assert.match(panel, /好/);
    assert.doesNotMatch(panel, /坏|undefined|null|123/);
    await p.key('e');
    await p.waitFor(`${TAB} === '二'`, 2000, 'entries 写错的分类也要能切过去');
  },

  async load_twice_builds_one_tabbar(p, { base }) {
    await p.open(base + page('home-ok', '&wikislow=1'));
    await waitHome(p);
    await p.key('ArrowDown');
    await p.key('Enter');
    await p.waitFor(ON, 2000, '第一次没有进图鉴');
    await p.key('Escape');
    await p.waitFor(`!${ON}`, 2000, '没有回首页');
    await p.key('Enter');
    await p.waitFor(`${ON} && !!document.querySelector('#wdesc h2')`, 3000, '第二次没有进图鉴');
    await sleep(600);
    assert.equal(await p.eval(`document.querySelectorAll('#codex .tabwrap').length`), 1, '标签栏只能有一个');
  },

  async missing_state(p, { base }) {
    await openWiki(p, base, '&wiki=missing');
    assert.match(await p.eval(NONE), /图鉴数据缺失/);
    assert.equal(await p.eval(COUNT), 0);
    await p.key('Escape');
    await p.waitFor(`!${ON}`, 2000, '缺数据时 Esc 也要能回首页');
  },

  async incompatible_state(p, { base }) {
    await openWiki(p, base, '&wiki=incompatible');
    assert.match(await p.eval(NONE), /需要新版启动器/);
  },

  async broken_state(p, { base }) {
    await openWiki(p, base, '&wiki=broken');
    assert.match(await p.eval(NONE), /图鉴数据损坏/);
  },

  async launch_view_leaves_wiki(p, { base }) {
    await openWiki(p, base);
    await p.eval(`window.__fakeEmit('launch-view', { phase: 'step', step: 'preflight' })`);
    await p.waitFor(`!${ON}`, 2000, '启动过场开始时应离开图鉴');
  },

  async grid_lr_stays_in_row(p, { base }) {
    await openWiki(p, base);
    await gotoCell(p, 4);
    await p.key('ArrowRight');
    assert.equal(await p.eval(FOCUS), 4, '行尾按右不换行');
    await gotoCell(p, 5);
    await p.key('ArrowLeft');
    assert.equal(await p.eval(FOCUS), 5, '行首按左不换行');
  },

  async grid_down_lands_on_last(p, { base }) {
    await openWiki(p, base);
    await gotoCell(p, 9);
    await p.key('ArrowDown');
    assert.equal(await p.eval(FOCUS), 11, '下一行不满时落到最后一格');
  },

  async grid_up_from_top_row_to_search(p, { base }) {
    await openWiki(p, base);
    await p.key('ArrowUp');
    assert.equal(await p.eval(ZONE), 1);
  },

  async filter_changes_list_stops_at_edge(p, { base }) {
    await openWiki(p, base);
    await p.key('ArrowUp');
    await p.key('ArrowUp');
    assert.equal(await p.eval(ZONE), 0);
    assert.equal(await p.eval(FILTER), '全部');
    await p.key('ArrowLeft');
    assert.equal(await p.eval(FILTER), '全部', '到边界不回绕');
    await p.key('ArrowRight');
    assert.equal(await p.eval(FILTER), '本版新增');
    assert.equal(await p.eval(COUNT), 1);
    assert.equal(await p.eval(NAME), '测试武器3');
  },

  async one_option_hides_filter_row(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '机制');
    assert.equal(await p.eval(`document.querySelector('#codex').classList.contains('one')`), true);
    await p.key('ArrowUp');
    assert.equal(await p.eval(ZONE), 1);
    await p.key('ArrowUp');
    assert.equal(await p.eval(ZONE), 1, '隐藏的显示行走不上去');
  },

  async search_filters_current(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '大剑');
    await p.waitFor(`${COUNT} === 4`, 2000, '查找没有过滤出 4 把大剑');
    assert.equal(await p.eval(ON), true);
  },

  async search_jumps_to_first_hit(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '护符');
    await p.waitFor(`${COUNT} === 0`, 2000, '武器里不该有结果');
    await p.key('Enter');
    await p.waitFor(`${TAB} === '护符'`, 2000, '结束输入后应跳到第一个有结果的分类');
    assert.equal(await p.eval(COUNT), 3);
    assert.equal(await p.eval(ZONE), 2);
  },

  async search_no_hit_stays(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '不存在的字');
    await p.key('Enter');
    await p.waitFor(NOT_EDITING, 2000, 'Enter 应结束输入');
    assert.match(await p.eval(NONE), /没有找到/);
    assert.equal(await p.eval(TAB), '武器');
  },

  async esc_cancels_search(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '大剑');
    await p.waitFor(`${COUNT} === 4`, 2000, '查找没有生效');
    await p.key('Escape');
    await p.waitFor(NOT_EDITING, 2000, 'Esc 应结束输入');
    assert.equal(await p.eval(QUERY), '');
    assert.equal(await p.eval(COUNT), 12);
    assert.equal(await p.eval(ON), true, 'Esc 只取消输入，不退出页面');
  },

  async right_click_cancels_search(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '大剑');
    await p.waitFor(`${COUNT} === 4`, 2000, '查找没有生效');
    const c = await p.center('#wname');
    await p.mouseMove(c.x, c.y);
    await p.click(c.x, c.y, { button: 'right' });
    await p.waitFor(NOT_EDITING, 2000, '右键应取消输入');
    assert.equal(await p.eval(QUERY), '');
    assert.equal(await p.eval(ON), true);
  },

  async ime_enter_keeps_editing(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await p.eval(`document.querySelector('#wq').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true, cancelable: true }))`);
    await sleep(100);
    assert.equal(await p.eval(`document.activeElement?.id`), 'wq', '组字中的 Enter 不结束输入');
  },

  async backspace_clears_search(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '大剑');
    await p.key('Enter');
    await p.waitFor(NOT_EDITING, 2000, 'Enter 应结束输入');
    for (let i = 0; i < 3 && (await p.eval(ZONE)) !== 1; i++) await p.key('ArrowUp');
    await p.key('Backspace');
    assert.equal(await p.eval(QUERY), '');
    assert.equal(await p.eval(COUNT), 12);
  },

  async none_tab_dimmed(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '大剑');
    await p.waitFor(`${COUNT} === 4`, 2000, '查找没有生效');
    assert.equal(await p.eval(has(`${TAB_CELL(2)}.none`)), true, '护符没有结果，标签要变暗');
    assert.equal(await p.eval(has(`${TAB_CELL(1)}.none`)), false, '当前分类不变暗');
  },

  async empty_category_text(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '法术');
    assert.match(await p.eval(NONE), /暂无法术条目/);
  },

  async large_grid_keeps_focus_visible(p, { base }) {
    await openWiki(p, base, '&wiki=large');
    const visible = `(() => {
      const f = document.querySelector('#wbox .wcell.f').getBoundingClientRect();
      const b = document.querySelector('#wbox').getBoundingClientRect();
      return f.top >= b.top && f.bottom <= b.bottom;
    })()`;
    for (let i = 0; i < 60; i++) {
      await p.key('ArrowDown');
      assert.equal(await p.eval(visible), true, `第 ${i + 1} 次按下后选中项跑出了可见范围`);
    }
    assert.equal(await p.eval(FOCUS), 300);
    await p.key('e');
    await p.key('q');
    await p.waitFor(visible, 2000, '切回来后选中项不在可见范围');
  },

  async filter_remembered_per_cat(p, { base }) {
    await openWiki(p, base);
    await p.key('ArrowUp');
    await p.key('ArrowUp');
    await p.key('ArrowRight');
    assert.equal(await p.eval(FILTER), '本版新增');
    await p.key('e');
    await p.key('q');
    await p.waitFor(`${FILTER} === '本版新增'`, 2000, '切回来筛选应保留');
  },

  async open_from_home_clears_search(p, { base }) {
    await openWiki(p, base);
    await startSearch(p);
    await typeText(p, '大剑');
    await p.key('Enter');
    await p.waitFor(NOT_EDITING, 2000, 'Enter 应结束输入');
    await p.key('Escape');
    await p.waitFor(`!${ON}`, 2000, '没有回首页');
    await sleep(560);
    await p.key('Enter');
    await p.waitFor(ON, 2000, '没有重新进图鉴');
    await p.waitFor(`${QUERY} === ''`, 2000, '从首页进来要清空查找');
    assert.equal(await p.eval(COUNT), 12);
  },
};
