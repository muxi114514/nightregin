// 存档页
import assert from 'node:assert/strict';
import { sleep } from './cdp.mjs';
import { TOAST, answer, calls, shot, waitDialog } from './screen-helpers.mjs';
import { DESC, ROWS, SAVES, SEL, VAL, openSys, selectRow } from './sys-helpers.mjs';

const A = '76561198000000001';
const B = '76561198000000002';
const BK = 'NR0000_2026-11-06_214005_auto.co2.bak';

async function enter(p) {
  await sleep(320);
  await p.key('Enter');
}

export const tests = {
  async saves_rows_order(p, { base }) {
    await openSys(p, base, SAVES);
    const rows = await p.eval(ROWS);
    assert.deepEqual(rows.slice(0, 4), ['backup', 'folder', 'suffix', 'account']);
    assert.ok(rows.includes('sep'));
    assert.ok(rows.includes(BK));
    assert.ok(rows.indexOf('sep') < rows.indexOf(BK));
    assert.equal(await p.eval(VAL('suffix')), 'co2');
    await sleep(500);
    await shot(p, 'sys_saves');
  },

  async saves_vanilla_row_only_when_available(p, { base }) {
    await openSys(p, base, SAVES);
    assert.ok(!(await p.eval(ROWS)).includes('vanilla'));
    await openSys(p, base, SAVES, '&saves=vanilla');
    assert.ok((await p.eval(ROWS)).includes('vanilla'));
    await selectRow(p, 'vanilla');
    await enter(p);
    await p.waitFor(`document.querySelector('.olist .row[data-id="vanilla"]') === null`, 2000, '复制后这一行应消失');
    assert.deepEqual((await calls(p, 'copyFromVanilla'))[0].args, { account: A });
  },

  async saves_restore_confirm_defaults_cancel(p, { base }) {
    await openSys(p, base, SAVES);
    await selectRow(p, BK);
    await enter(p);
    const d = await waitDialog(p);
    assert.match(d.msg, /2026-11-06 21:40:05/);
    assert.equal(d.sel, 1);
    await answer(p, 'Enter');
    await sleep(100);
    assert.equal((await calls(p, 'restoreBackup')).length, 0);
  },

  async saves_restore_confirm_then_calls(p, { base }) {
    await openSys(p, base, SAVES);
    await selectRow(p, BK);
    await enter(p);
    await waitDialog(p);
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`(window.__calls || []).some((c) => c.name === 'restoreBackup')`, 2000, '没调恢复');
    assert.deepEqual((await calls(p, 'restoreBackup'))[0].args, { account: A, name: BK });
  },

  async saves_restore_refused_when_running(p, { base }) {
    await openSys(p, base, SAVES, '&saves=running');
    await selectRow(p, BK);
    await enter(p);
    await p.waitFor(`${TOAST}.includes('不能恢复')`, 1500, '运行中要拒绝');
    assert.equal((await calls(p, 'restoreBackup')).length, 0);
  },

  async saves_prerestore_on_top_after_restore(p, { base }) {
    await openSys(p, base, SAVES);
    await selectRow(p, BK);
    await enter(p);
    await waitDialog(p);
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`[...document.querySelectorAll('.olist .row')].some((r) => r.dataset.id.includes('prerestore'))`, 2000, '恢复前备份没出现');
    const rows = await p.eval(ROWS);
    assert.match(rows[rows.indexOf('sep') + 1], /prerestore/);
  },

  async saves_manual_backup_warns_when_running(p, { base }) {
    await openSys(p, base, SAVES, '&saves=running');
    await enter(p);
    const d = await waitDialog(p);
    assert.match(d.msg, /可能不完整/);
    assert.equal(d.sel, 1);
    await answer(p, 'ArrowLeft');
    await p.key('Enter');
    await p.waitFor(`${TOAST}.includes('已备份')`, 2000, '确认后应备份');
    const c = await calls(p, 'backupNow');
    assert.deepEqual(c.map((x) => x.args.force), [false, true]);
  },

  async saves_backup_result_toasts(p, { base }) {
    await openSys(p, base, SAVES);
    await enter(p);
    await p.waitFor(`${TOAST}.includes('已备份')`, 1500, '没提示已备份');
    assert.ok((await p.eval(ROWS)).some((r) => r.includes('manual')), '列表要刷新');
  },

  async saves_backup_unchanged_toast(p, { base }) {
    await openSys(p, base, SAVES, '&saves=unchanged');
    await enter(p);
    await p.waitFor(`${TOAST}.includes('存档没有变化')`, 1500, '没变化要提示');
  },

  async saves_suffix_align_when_mismatch(p, { base }) {
    await openSys(p, base, SAVES, '&saves=mismatch');
    assert.equal(await p.eval(VAL('suffix')), '不一致');
    await selectRow(p, 'suffix');
    await enter(p);
    await p.waitFor(`${VAL('suffix')} === 'co2'`, 1500, '对齐后应刷新');
    assert.equal((await calls(p, 'alignSuffix')).length, 1);
  },

  async saves_suffix_consistent_enter_noop(p, { base }) {
    await openSys(p, base, SAVES);
    await selectRow(p, 'suffix');
    await enter(p);
    await sleep(120);
    assert.equal((await calls(p, 'alignSuffix')).length, 0);
  },

  async saves_account_switch_keeps_row(p, { base }) {
    await openSys(p, base, SAVES, '&saves=two');
    await selectRow(p, 'account');
    assert.match(await p.eval(VAL('account')), /…0001/);
    await p.key('ArrowRight');
    await p.waitFor(`${VAL('account')}.includes('…0002')`, 1500, '账号没切换');
    assert.equal(await p.eval(SEL), 'account');
    assert.deepEqual((await calls(p, 'setSaveAccount')).at(-1).args, { account: B });
    assert.deepEqual((await calls(p, 'getSaves')).at(-1).args, { account: B });
  },

  async saves_account_not_logged_in_note(p, { base }) {
    await openSys(p, base, SAVES, '&saves=two');
    await selectRow(p, 'account');
    await p.key('ArrowRight');
    await p.waitFor(`${DESC}.includes('另一个账号')`, 1500, '要说明备份按登录账号');
  },

  async saves_empty_list_text(p, { base }) {
    await openSys(p, base, SAVES, '&saves=empty');
    assert.ok((await p.eval(`document.querySelector('.olist').textContent`)).includes('还没有备份'));
  },
};
