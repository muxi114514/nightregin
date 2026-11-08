// 图鉴的说明面板：字段、手写标记、详细说明、几率
import assert from 'node:assert/strict';
import { HINTS, NAME, PANEL, gotoCell, gotoTab, openWiki } from './wiki-helpers.mjs';

const text = (sel) => `(document.querySelector('#wdesc ${sel}')?.textContent ?? '')`;
const exists = (sel) => `(!!document.querySelector('#wdesc ${sel}'))`;
const src = (sel) => `(document.querySelector(${JSON.stringify(sel)})?.getAttribute('src') ?? '')`;
const NOTE = '基础几率，未计入防重复与地变保护';
const FOOTER = '作者补充，非游戏数据';
const SCROLL_TOP = `document.querySelector('#wdesc .wbody').scrollTop`;

async function enemy(p, base, cell, extra = '') {
  await openWiki(p, base, extra);
  await gotoTab(p, '敌人');
  await gotoCell(p, cell);
}

export const tests = {
  async panel_shows_fields(p, { base }) {
    await openWiki(p, base);
    assert.equal(await p.eval(NAME), '测试武器1');
    assert.match(await p.eval(text('.meta')), /GA 移植/);
    assert.match(await p.eval(text('.kv')), /基础攻击力/);
    assert.match(await p.eval(text('.wbody')), /第 1 把武器的说明/);
    assert.match(await p.eval(text('.wsrc')), /获取方式.*首领掉落球/);
  },

  async sections_show_title_and_text(p, { base }) {
    await openWiki(p, base);
    const titles = await p.eval(`[...document.querySelectorAll('#wdesc .wsec dt')].map((e) => e.textContent)`);
    assert.deepEqual(titles, ['战技 · 狮子斩', '还可能附带（2 种）']);
    assert.match(await p.eval(text('.wsec dd')), /朝前方跳跃/);
    assert.equal(await p.eval(exists('.wsec + .lore, .wsec ~ .lore')), true, '传说排在战技后面');
  },

  async sections_stay_in_detail_view(p, { base }) {
    await openWiki(p, base);
    await p.key('r');
    await p.waitFor(`document.querySelector('#wdesc').classList.contains('det')`, 2000, '没有进入详细说明');
    assert.notEqual(await p.eval(`getComputedStyle(document.querySelector('#wdesc .wsec')).display`), 'none');
  },

  async entry_without_sections_has_no_block(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '护符');
    assert.equal(await p.eval(exists('.wsec')), false);
  },

  async malformed_sections_skipped(p, { base }) {
    await openWiki(p, base, '&wiki=malformed');
    const titles = await p.eval(`[...document.querySelectorAll('#wdesc .wsec dt')].map((e) => e.textContent)`);
    assert.deepEqual(titles, ['好段落']);
    assert.doesNotMatch(await p.eval(PANEL), /undefined|null|坏段落/);
  },

  async partial_entry_renders(p, { base }) {
    await openWiki(p, base, '&wiki=odd');
    assert.equal(await p.eval(NAME), '只有名字');
    assert.doesNotMatch(await p.eval(PANEL), /undefined|null|NaN/);
  },

  async no_source_hides_row(p, { base }) {
    await openWiki(p, base);
    await gotoCell(p, 11);
    assert.equal(await p.eval(NAME), '测试武器12');
    assert.equal(await p.eval(exists('.wsrc')), false);
  },

  async source_label_from_data(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '大地图');
    assert.match(await p.eval(text('.wsrc')), /出现/);
  },

  async note_never_rendered(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '机制');
    assert.equal(await p.eval(NAME), '属性分配');
    assert.equal((await p.eval(`document.querySelector('#codex').textContent`)).includes('内部备注XYZ'), false);
  },

  async hand_is_italic_with_footer(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '机制');
    assert.equal(await p.eval(exists('.wbody p.ph')), true);
    assert.equal((await p.eval(PANEL)).includes(FOOTER), true);
  },

  async hand_sub_is_italic(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '机制');
    assert.equal(await p.eval(exists('.meta.ph')), true);
    assert.equal(await p.eval(`!!document.querySelector('#wbox .row.sel .kd.ph')`), true);
  },

  async real_data_has_no_footer(p, { base }) {
    await openWiki(p, base);
    await gotoCell(p, 1);
    assert.equal((await p.eval(PANEL)).includes(FOOTER), false);
  },

  async new_tag_in_panel(p, { base }) {
    await openWiki(p, base);
    await gotoCell(p, 2);
    assert.equal(await p.eval(exists('.wtag .nw')), true);
    assert.match(await p.eval(text('.wtag')), /1\.9\.5 新增/);
    await gotoCell(p, 1);
    assert.equal(await p.eval(exists('.wtag')), false);
  },

  async missing_image_hidden(p, { base }) {
    await openWiki(p, base, '&wiki=odd');
    const gone = (sel) => `(() => { const i = document.querySelector(${JSON.stringify(sel)}); return !!i && getComputedStyle(i).display === 'none'; })()`;
    await p.waitFor(gone('#wbox img.ic'), 3000, '条目区的破图标没有隐藏');
    await p.waitFor(gone('#wdesc .pic img'), 3000, '面板的破图标没有隐藏');
  },

  async r_expands_and_scrolls(p, { base }) {
    await openWiki(p, base);
    await p.key('r');
    await p.waitFor(`document.querySelector('#wdesc').classList.contains('det')`, 2000, 'R 没有展开');
    assert.equal(await p.eval(SCROLL_TOP), 0);
    await p.key('ArrowDown');
    await p.waitFor(`${SCROLL_TOP} > 0`, 2000, '↓ 没有滚动正文');
    assert.equal(await p.eval(NAME), '测试武器1', '详细说明里 ↓ 不换条目');
    await p.key('r');
    await p.waitFor(`!document.querySelector('#wdesc').classList.contains('det')`, 2000, 'R 没有收起');
  },

  async esc_closes_detail_only(p, { base }) {
    await openWiki(p, base);
    await p.key('r');
    await p.waitFor(`document.querySelector('#wdesc').classList.contains('det')`, 2000, 'R 没有展开');
    await p.key('Escape');
    await p.waitFor(`!document.querySelector('#wdesc').classList.contains('det')`, 2000, 'Esc 没有收起');
    assert.equal(await p.eval(`!!document.querySelector('#codex.on')`), true, '第一次 Esc 只收起详细说明');
  },

  async r_hint_only_when_overflow(p, { base }) {
    await openWiki(p, base);
    assert.match(await p.eval(HINTS), /详细说明/);
    await gotoCell(p, 1);
    assert.doesNotMatch(await p.eval(HINTS), /详细说明/);
  },

  async odds_forced_level(p, { base }) {
    await enemy(p, base, 6);
    assert.equal(await p.eval(NAME), '卡莉果');
    assert.match(await p.eval(text('.wsrc')), /每次出征 9%/);
  },

  async odds_open_level(p, { base }) {
    await enemy(p, base, 6, '&odds=open');
    assert.match(await p.eval(text('.wsrc')), /深夜 1～5 层 5 \/ 6 \/ 7 \/ 8 \/ 9%/);
  },

  async odds_open_uniform(p, { base }) {
    await enemy(p, base, 0, '&odds=open');
    assert.match(await p.eval(text('.wsrc')), /每次出征 10%/);
  },

  async odds_map(p, { base }) {
    await openWiki(p, base);
    await gotoTab(p, '大地图');
    assert.match(await p.eval(text('.wsrc')), /每次出征 20%/);
  },

  async odds_off_falls_back(p, { base }) {
    await enemy(p, base, 0, '&odds=off');
    assert.match(await p.eval(text('.wsrc')), /远征目标/);
    const panel = await p.eval(PANEL);
    assert.equal(panel.includes(NOTE), false);
    assert.doesNotMatch(panel, /undefined|NaN/);
  },

  async odds_note_shown(p, { base }) {
    await enemy(p, base, 0);
    assert.equal(await p.eval(text('.wnote')), NOTE);
  },

  async ever_100_0_partial(p, { base }) {
    await enemy(p, base, 0);
    assert.match(await p.eval(text('.ever')), /以永夜王形态出现：永夜之王 格拉狄乌斯/);
    await gotoCell(p, 1);
    assert.match(await p.eval(text('.ever')), /以普通形态出现/);
    await gotoCell(p, 2);
    assert.match(await p.eval(text('.ever')), /66% 以永夜王形态出现/);
  },

  async ever_open_level(p, { base }) {
    await enemy(p, base, 2, '&odds=open');
    assert.match(await p.eval(text('.ever')), /深夜 1～5 层 25 \/ 33 \/ 40 \/ 50 \/ 66% 以永夜王形态出现/);
  },

  async ever_off(p, { base }) {
    await enemy(p, base, 0, '&odds=off');
    assert.match(await p.eval(text('.ever')), /按游戏原规则出现/);
  },

  async no_lord_no_ever_line(p, { base }) {
    await enemy(p, base, 7);
    assert.equal(await p.eval(NAME), '黑夜王');
    assert.equal(await p.eval(exists('.ever')), false);
  },

  async portrait_round_frame(p, { base }) {
    await enemy(p, base, 7);
    assert.equal(await p.eval(exists('.pic.rd')), true, '没有永夜形态的夜王也是圆头像');
    assert.equal(await p.eval(`!!document.querySelector('#wbox .wcell .i.pt')`), true);
  },

  async item_square_frame(p, { base }) {
    await openWiki(p, base);
    assert.equal(await p.eval(exists('.pic.box')), true);
    assert.equal(await p.eval(`!!document.querySelector('#wbox .wcell .i.pt')`), false);
  },

  async ever_icon_when_100(p, { base }) {
    await enemy(p, base, 0);
    assert.match(await p.eval(src('#wdesc .pic img')), /img\/nl_0_ever\.webp/);
    assert.match(await p.eval(src('#wbox .wcell[data-i="0"] .i')), /img\/nl_0_ever\.webp/);
    await gotoCell(p, 1);
    assert.match(await p.eval(src('#wdesc .pic img')), /img\/nl_1\.webp/);
  },
};
