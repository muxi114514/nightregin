// 交互规则探测：在 probe.html（三项菜单 + 对话框按钮 + 输入框）上发真实 CDP 键鼠，核对页面反应
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';

const PAGE = 'probe.html?fake=home-ok';
const ITEM = (n) => `.tmenu button[data-i="${n - 1}"]`;
/** 当前选中第几项（1 起）；没有选中为 0 */
const SELECTED = `(() => { const b = document.querySelector('.tmenu button.sel'); return b ? +b.dataset.i + 1 : 0; })()`;
const DIALOG_OPEN = `!!document.querySelector('.dlg.on')`;
/** 对话框当前选中第几个选项（0 起）；没有为 -1 */
const DIALOG_SEL = `(() => { const b = document.querySelector('.dlg .opt.sel'); return b ? +b.dataset.i : -1; })()`;

/** 点开对话框，返回看到它打开的时刻 */
async function openDialog(p, selector = '#probe-dialog') {
  const btn = await p.center(selector);
  await p.click(btn.x, btn.y);
  await p.waitFor(DIALOG_OPEN, 2000, '对话框没有打开');
  return Date.now();
}

/** 点一个按键提示；part 为贴图上的横向位置比例（0~1），不给时点按钮中心 */
async function clickHint(p, key, part) {
  const sel = `.dlg .keys [data-k="${key}"]`;
  if (part === undefined) {
    const c = await p.center(sel);
    return p.click(c.x, c.y);
  }
  const r = await p.eval(`(() => { const img = document.querySelector('${sel} img'); if (!img) return null;
    const r = img.getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`);
  if (!r) throw new Error(`找不到可点的按键提示：${key}`);
  return p.click(r.x + r.w * part, r.y + r.h / 2);
}

/** 钩子记下的最近一次匹配事件（分发完后读 defaultPrevented）应为 want */
async function expectPrevented(p, label, match, want) {
  await sleep(150);
  const list = await p.eval('window.__prevented');
  // 没拦住的刷新、前进后退会把页面换掉，钩子记录跟着没了
  assert.ok(Array.isArray(list), `页面被刷新或导航走了：${label}`);
  const hit = list.filter(match);
  assert.ok(hit.length, `没记到事件：${label}`);
  assert.equal(hit[hit.length - 1].prevented, want, label);
}

export const tests = {
  async hover_requires_real_move(p, { base }) {
    await p.open(base + PAGE);
    const c = await p.center(ITEM(3));
    await p.mouseMove(c.x, c.y);
    await p.waitFor(`${SELECTED} === 3`, 2000, '鼠标移到第 3 项后没有选中 3');
    await p.key('ArrowUp');
    assert.equal(await p.eval(SELECTED), 2, '按 ↑ 后应选中 2');
    await p.mouseMove(c.x, c.y);
    await sleep(200);
    assert.equal(await p.eval(SELECTED), 2, '同一坐标再发 mouseMoved 不应改选中');
    await p.mouseMove(c.x + 2, c.y);
    await p.waitFor(`${SELECTED} === 3`, 2000, '移动 2px 后应重新选中 3');
    assert.deepEqual(await p.eval('window.__activated'), [], '悬停只选中，不应执行');
  },

  async click_activates(p, { base }) {
    await p.open(base + PAGE);
    await p.key('ArrowDown');
    await p.key('ArrowDown');
    assert.equal(await p.eval(SELECTED), 3);
    const c = await p.center(ITEM(1));
    await p.click(c.x, c.y);
    await sleep(200);
    assert.equal(await p.eval(SELECTED), 1, '单击第 1 项后应选中 1');
    assert.deepEqual(await p.eval('window.__activated'), [1], '单击应同 Enter 执行被点的项');
    // 关掉对话框后 500ms 内单击菜单项：丢弃，不选中也不执行
    const openedAt = await openDialog(p);
    await sleep(400 - (Date.now() - openedAt));
    const pressedAt = Date.now();
    await p.key('Escape');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, 'Esc 应关闭对话框');
    const closedAt = Date.now();
    const second = await p.center(ITEM(2));
    await p.click(second.x, second.y);
    const early = Date.now() - pressedAt;
    assert.ok(early < 400, `关闭后的第一下单击发得太晚（${early}ms），测不到 500ms 冷却`);
    await sleep(80);
    assert.equal(await p.eval(SELECTED), 1, '关闭对话框后 500ms 内的单击不应选中');
    assert.deepEqual(await p.eval('window.__activated'), [1], '关闭对话框后 500ms 内的单击不应执行');
    await sleep(700 - (Date.now() - closedAt));
    await p.click(second.x, second.y);
    await sleep(150);
    assert.equal(await p.eval(SELECTED), 2);
    assert.deepEqual(await p.eval('window.__activated'), [1, 2], '冷却过后单击第 2 项应执行它');
  },

  async repeat_enter_ignored(p, { base }) {
    await p.open(base + PAGE);
    await p.key('Enter', { repeat: true });
    await sleep(150);
    assert.deepEqual(await p.eval('window.__activated'), [], '按住产生的重复 Enter 不应执行');
    // 对照：普通 Enter 会执行当前项
    await p.key('Enter');
    await sleep(150);
    assert.deepEqual(await p.eval('window.__activated'), [1], '普通 Enter 应执行第 1 项');
  },

  async click_guard_500ms(p, { base }) {
    await p.open(base + PAGE);
    const btn = await p.center('#probe-dialog');
    await p.click(btn.x, btn.y);
    await p.waitFor(DIALOG_OPEN, 2000, '对话框没有打开');
    const openedAt = Date.now();
    const ok = await p.center('.dlg .opt[data-i="0"]');
    await sleep(200 - (Date.now() - openedAt));
    await p.click(ok.x, ok.y);
    await sleep(80);
    assert.ok(await p.eval(DIALOG_OPEN), '打开后 200ms 的点击应被丢弃');
    assert.equal(await p.eval('window.__dialogResult'), undefined);
    await sleep(700 - (Date.now() - openedAt));
    await p.click(ok.x, ok.y);
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, '700ms 后点「确定」应关闭对话框');
    assert.equal(await p.eval('window.__dialogResult'), true, '点「确定」应返回 true');
  },

  async enter_blocked_in_dialog_and_input(p, { base }) {
    await p.open(base + PAGE);
    const btn = await p.center('#probe-dialog');
    await p.click(btn.x, btn.y);
    await p.waitFor(DIALOG_OPEN, 2000, '对话框没有打开');
    await sleep(400);
    await p.key('Enter');
    await sleep(150);
    assert.deepEqual(await p.eval('window.__activated'), [], '对话框开着时 Enter 不应触发菜单项');
    assert.ok(!(await p.eval(DIALOG_OPEN)), 'Enter 应交给对话框（确定并关闭）');
    assert.equal(await p.eval('window.__dialogResult'), true, 'Enter 应按选中的「确定」返回 true');

    await p.eval('document.getElementById("probe-input").focus()');
    await p.key('Enter');
    await sleep(150);
    assert.deepEqual(await p.eval('window.__activated'), [], '焦点在输入框时 Enter 不应触发菜单项');
    // 对照：离开输入框后 Enter 会执行
    await p.eval('document.getElementById("probe-input").blur()');
    await p.key('Enter');
    await sleep(150);
    assert.deepEqual(await p.eval('window.__activated'), [1], '离开输入框后 Enter 应执行第 1 项');
  },

  async enter_guard_after_dialog_open(p, { base }) {
    await p.open(base + PAGE);
    const openedAt = await openDialog(p);
    // 在冷却中段（约 200ms）按，冷却短于 200ms 也测得出
    await sleep(200 - (Date.now() - openedAt));
    await p.key('Enter');
    const early = Date.now() - openedAt;
    assert.ok(early < 260, `第一下 Enter 发得太晚（${early}ms），测不到 300ms 冷却`);
    await sleep(60);
    assert.ok(await p.eval(DIALOG_OPEN), '对话框打开后 300ms 内的 Enter 应被忽略');
    assert.equal(await p.eval('window.__dialogResult'), undefined);
    await sleep(400 - (Date.now() - openedAt));
    await p.key('Enter');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, '300ms 之后的 Enter 应确定并关闭对话框');
    assert.equal(await p.eval('window.__dialogResult'), true);
  },

  async destructive_confirm_defaults_to_cancel(p, { base }) {
    await p.open(base + PAGE);
    await openDialog(p, '#probe-danger');
    assert.equal(await p.eval(DIALOG_SEL), 1, '危险操作的确认框应默认选中第 2 项');
    assert.equal(await p.eval(`document.querySelector('.dlg .opt.sel').textContent`), '取消');
    await sleep(400);
    await p.key('Enter');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, 'Enter 应关闭对话框');
    assert.equal(await p.eval('window.__dialogResult'), false, '默认项上按 Enter 应返回 false');
  },

  async key_hints_clickable(p, { base }) {
    await p.open(base + PAGE);
    await openDialog(p);
    await sleep(600);
    assert.equal(await p.eval(DIALOG_SEL), 0);
    await clickHint(p, 'ArrowRight', 0.75);
    await sleep(100);
    assert.equal(await p.eval(DIALOG_SEL), 1, '点 ←→ 贴图右半边应同 →');
    await clickHint(p, 'ArrowRight', 0.25);
    await sleep(100);
    assert.equal(await p.eval(DIALOG_SEL), 0, '点 ←→ 贴图左半边应同 ←');
    await clickHint(p, 'Enter');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, '点 Enter 提示应确定并关闭对话框');
    assert.equal(await p.eval('window.__dialogResult'), true);
    // 关闭后 500ms 内的点击会被丢弃
    await sleep(600);
    await openDialog(p);
    await sleep(600);
    await clickHint(p, 'Escape');
    await p.waitFor(`!${DIALOG_OPEN}`, 2000, '点 Esc 提示应取消并关闭对话框');
    assert.equal(await p.eval('window.__dialogResult'), false);
  },

  async escape_and_right_click_go_back(p, { base }) {
    await p.open(base + PAGE);
    await p.key('Escape');
    await sleep(150);
    assert.equal(await p.eval('window.__back'), 1, 'Esc 应返回');
    await p.click(160, 520, { button: 'right' });
    await sleep(150);
    assert.equal(await p.eval('window.__back'), 2, '右键应返回');
    await expectPrevented(p, '右键菜单应被阻止', (e) => e.type === 'contextmenu', true);
  },

  async browser_keys_prevented(p, { base }) {
    await p.open(base + PAGE);
    await p.key('f', { ctrl: true });
    await expectPrevented(p, 'Ctrl+F', (e) => e.type === 'keydown' && e.key === 'f' && e.ctrl, true);
    await p.key('F5');
    await expectPrevented(p, 'F5', (e) => e.type === 'keydown' && e.key === 'F5', true);
    await p.key('Tab');
    await expectPrevented(p, 'Tab', (e) => e.type === 'keydown' && e.key === 'Tab', true);
    await p.wheel(600, 360, -120, { ctrl: true });
    await expectPrevented(p, 'Ctrl+滚轮', (e) => e.type === 'wheel' && e.ctrl, true);
    // 对照：普通滚轮不拦
    await p.wheel(600, 360, -120);
    await expectPrevented(p, '普通滚轮不应被拦', (e) => e.type === 'wheel' && !e.ctrl, false);
  },

  async side_buttons_prevented(p, { base }) {
    await p.open(base + PAGE);
    await p.click(160, 520, { button: 'back' });
    await expectPrevented(p, '侧键 3 的 mouseup', (e) => e.type === 'mouseup' && e.button === 3, true);
    await p.click(160, 520, { button: 'forward' });
    await expectPrevented(p, '侧键 4 的 mouseup', (e) => e.type === 'mouseup' && e.button === 4, true);
    // 对照：左键 mouseup 不拦
    await p.click(160, 520);
    await expectPrevented(p, '左键 mouseup 不应被拦', (e) => e.type === 'mouseup' && e.button === 0, false);
  },
};
