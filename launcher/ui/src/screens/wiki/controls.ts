// 图鉴左上角的「显示」「查找」两行，和底部按键提示
import { el } from '../../ui/dom';
import type { Hint } from '../../ui/hints';
import { arrows } from '../../ui/rows';
import type { WikiModel } from './model';
import { WIKI_TEXT } from './texts';

const SEARCH_MAX = 16;

/** 「显示」行和「查找」行；data-z 是焦点区编号 */
export function renderControls(m: WikiModel, editing: boolean): HTMLElement[] {
  const opts = m.options();
  const k = m.filterIndex();
  const filter = el('li', 'row');
  filter.dataset.z = '0';
  filter.append(Object.assign(el('span', 'lb'), { textContent: WIKI_TEXT.show }), arrows(opts[k] ?? '', k <= 0, k >= opts.length - 1));
  const search = el('li', editing ? 'row edit' : 'row');
  search.dataset.z = '1';
  const box = el('span', 'val');
  const field = el('input', 'wq');
  field.id = 'wq';
  field.maxLength = SEARCH_MAX;
  field.placeholder = WIKI_TEXT.searchHint;
  field.value = m.query;
  field.readOnly = !editing;
  field.tabIndex = -1;
  field.spellcheck = false;
  field.autocomplete = 'off';
  box.append(field);
  search.append(Object.assign(el('span', 'lb'), { textContent: WIKI_TEXT.search }), box);
  return [filter, search];
}

export interface HintState {
  zone: 0 | 1 | 2;
  editing: boolean;
  detail: boolean;
  grid: boolean;
  hasQuery: boolean;
  /** 当前条目能进永夜王设置 */
  lord: boolean;
  /** 正文放不下，能展开详细说明 */
  overflow: boolean;
}

export function wikiHints(s: HintState): Hint[] {
  if (s.detail) return [{ icon: 'key_ud', label: WIKI_TEXT.scroll }, { icon: 'key_r', label: WIKI_TEXT.collapse, key: 'r' }];
  if (s.editing) return [{ icon: 'key_enter', label: WIKI_TEXT.done, key: 'Enter' }, { icon: 'key_esc', label: WIKI_TEXT.cancel, key: 'Escape' }];
  const hints: Hint[] = [{ icon: s.zone === 2 && s.grid ? 'key_udlr' : 'key_ud', label: WIKI_TEXT.select }];
  if (s.zone === 0) hints.push({ icon: 'key_lr', label: WIKI_TEXT.adjust, key: 'ArrowRight', alt: 'ArrowLeft' });
  if (s.zone === 1) {
    hints.push({ icon: 'key_enter', label: WIKI_TEXT.input, key: 'Enter' });
    if (s.hasQuery) hints.push({ icon: 'key_bs', label: WIKI_TEXT.clear, key: 'Backspace' });
  }
  if (s.zone === 2 && s.lord) hints.push({ icon: 'key_enter', label: WIKI_TEXT.lordSettings, key: 'Enter' });
  if (s.overflow) hints.push({ icon: 'key_r', label: WIKI_TEXT.detail, key: 'r' });
  hints.push({ icon: 'key_esc', label: WIKI_TEXT.back, key: 'Escape' });
  return hints;
}
