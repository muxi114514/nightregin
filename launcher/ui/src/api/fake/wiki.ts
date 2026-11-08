// 图鉴的假数据（探测用）：&wiki=missing|incompatible|broken|large|manycats|odd，&odds=open|off
import { asset } from '../../ui/dom';
import type { Api, WikiCategory, WikiEntry, WikiOdds, WikiView } from '../types';

type WikiApi = Pick<Api, 'getWiki' | 'getWikiOdds' | 'wikiImageUrl'>;

const LORD_NAMES = ['格拉狄乌斯', '艾德雷', '格诺斯塔', '玛利斯', '利普拉', '弗格尔', '卡莉果', '黑夜王', '哈尔莫妮亚'];
const LORD_KEYS = ['Gladius', 'Adel', 'Gnoster', 'Maris', 'Libra', 'Fulghor', 'Caligo', 'Heolstor', 'Harmonia', 'Straghess'];
const EVER_KEYS = ['Gladius', 'Adel', 'Gnoster', 'Maris', 'Libra', 'Fulghor', 'Caligo', 'Harmonia'];
const LONG_LORE = Array.from({ length: 80 }, (_, i) => `很长的传说第 ${i + 1} 行，这段文字足够让说明面板溢出。`).join('\n');
const WEAPON_FILTERS = ['GA 移植', '刀', '大剑'];
/** &wikislow=1 时读数据要多久 */
const SLOW_WIKI_MS = 400;

function weapon(i: number): WikiEntry {
  const blade = i <= 8;
  const group = blade ? '刀' : '大剑';
  return {
    id: `weapon-${i}`,
    name: `测试武器${i}`,
    sub: `${group} · ${i <= 4 ? 'GA 移植' : '原版'}`,
    icon: `img/w_${i}.webp`,
    filters: i <= 4 ? ['GA 移植', group] : [group],
    stats: [{ k: '类别', v: group }, { k: '基础攻击力', v: `物理 ${100 + i}` }],
    desc: `第 ${i} 把武器的说明。`,
    lore: i === 1 ? LONG_LORE : `第 ${i} 把武器的传说。`,
    sections: i === 1 ? [{ t: '战技 · 狮子斩', v: '朝前方跳跃，一边旋身一边敲击。' }, { t: '还可能附带（2 种）', v: '贯穿、突刺' }] : undefined,
    source: i === 12 ? undefined : '首领掉落球',
  };
}

function lord(row: number): WikiEntry {
  const grid = row <= 6 ? row : row === 8 ? 7 : undefined;
  return {
    id: `nightlord-${row}`,
    name: LORD_NAMES[row],
    sub: '远征 · 夜王',
    icon: `img/nl_${row}.webp`,
    portrait: true,
    filters: [row < 7 ? '夜王' : row === 7 ? '黑夜王' : 'DLC 夜王'],
    stats: [{ k: '有利属性', v: '火', h: true }],
    desc: `${LORD_NAMES[row]}的手写简介。`,
    source: '远征目标',
    hand: ['desc'],
    chance: { table: 'nightlord', key: LORD_KEYS[row] },
    ...(grid === undefined ? {} : { lord: grid, everIcon: `img/nl_${row}_ever.webp`, everName: `永夜之王 ${LORD_NAMES[row]}` }),
  };
}

const normal = (): WikiCategory[] => [
  { id: 'weapon', name: '武器', icon: 'img/cat_weapon.webp', layout: 'grid', order: 10, filters: WEAPON_FILTERS, entries: Array.from({ length: 12 }, (_, i) => weapon(i + 1)) },
  {
    id: 'talisman', name: '护符', icon: 'img/cat_talisman.webp', layout: 'grid', order: 20,
    entries: [1, 2, 3].map((i) => ({ id: `talisman-${i}`, name: `测试护符${i}`, sub: '护符', icon: `img/t_${i}.webp`, desc: `第 ${i} 个护符。`, source: '宝箱' })),
  },
  { id: 'spell', name: '法术', icon: 'img/cat_spell.webp', layout: 'grid', order: 30, emptyText: ['暂无法术条目', '整合包新增的法术会列在这里'], entries: [] },
  {
    id: 'map', name: '大地图', icon: 'img/cat_map.webp', layout: 'thumbs', order: 40, sourceLabel: '出现',
    entries: [
      { id: 'map-crater', name: '火山口', sub: '特异地形', icon: 'img/map_crater.webp', desc: '地形说明。', source: '偶尔出现', chance: { table: 'map', key: 'TheCrater' } },
      { id: 'map-lake', name: '湖沼', sub: '可攻略地点', icon: 'img/map_lake.webp', desc: '湖沼说明。', source: '地图据点', hand: ['source'] },
    ],
  },
  {
    id: 'enemy', name: '敌人', icon: 'img/cat_enemy.webp', layout: 'grid', order: 50, sourceLabel: '出现', filters: ['夜王', '黑夜王', 'DLC 夜王'],
    entries: [0, 1, 2, 3, 4, 5, 6, 7, 8].map(lord),
  },
  {
    id: 'mechanic', name: '机制', icon: 'img/cat_mech.webp', layout: 'list', order: 60, sourceLabel: '来源',
    entries: [
      { id: 'mech-1', name: '属性分配', sub: '自选成长', icon: 'img/mech_1.webp', desc: '手写的机制说明。', source: '整合包自带模组', hand: ['name', 'sub', 'desc', 'source'], note: '内部备注XYZ' } as WikiEntry,
      { id: 'mech-2', name: '词条继承', sub: '转移词条', icon: 'img/mech_2.webp', desc: '游戏数据说明。' },
    ],
  },
  {
    id: 'skill', name: '战技', icon: 'img/cat_skill.webp', layout: 'grid', order: 70,
    entries: [{ id: 'skill-100', name: '狮子斩', sub: '通用战技 · 原版', icon: 'img/a_8300.webp', stats: [{ k: '消耗专注', v: '16' }], desc: '朝前方跳跃。' }],
  },
];

function categories(kind: string | null): WikiCategory[] {
  if (kind === 'large') {
    return [{ id: 'weapon', name: '武器', layout: 'grid', order: 10, filters: WEAPON_FILTERS, entries: Array.from({ length: 521 }, (_, i) => weapon(i + 1)) }, ...normal().slice(1)];
  }
  if (kind === 'manycats') {
    return Array.from({ length: 14 }, (_, i) => ({
      id: `cat-${i}`, name: `分类${i + 1}`, layout: 'grid', order: i,
      entries: [{ id: `cat-${i}-a`, name: `条目${i}A` }, { id: `cat-${i}-b`, name: `条目${i}B` }],
    }));
  }
  if (kind === 'malformed') {
    return [
      {
        id: 'bad', name: '坏数据', layout: 'grid',
        filters: 'oops', hand: 3,
        entries: [
          { id: 'm-1', name: '缺值', stats: [{ k: '坏' }, { k: '好', v: '值' }, 7], desc: 123, lore: null, filters: 'oops', hand: 'desc', sections: [{ t: '好段落', v: '内容' }, { t: '坏段落' }, 7] },
          { id: 'm-2' },
          'notobject',
        ],
      } as unknown as WikiCategory,
      { id: 'c2', name: '二', layout: 'list', entries: 'nope' } as unknown as WikiCategory,
      5 as unknown as WikiCategory,
      { id: 'noname' } as unknown as WikiCategory,
    ];
  }
  if (kind === 'odd') {
    return [{ id: 'odd', name: '奇怪分类', layout: 'hex', entries: [{ id: 'odd-1', name: '只有名字', icon: 'img/none.webp', foo: 1 } as WikiEntry] }];
  }
  return normal();
}

function view(q: URLSearchParams): WikiView {
  const kind = q.get('wiki');
  if (kind === 'missing') return { state: 'missing' };
  if (kind === 'incompatible') return { state: 'incompatible', format: 2 };
  if (kind === 'broken') return { state: 'broken', message: 'expected value at line 1' };
  return { state: 'ok', data: { format: 1, version: '1.9.5', categories: categories(kind) }, newIds: ['weapon-3'] };
}

function odds(q: URLSearchParams): WikiOdds {
  const kind = q.get('odds');
  if (kind === 'off') return { level: 5, nightlords: null, maps: null, everdark: null };
  const nightlords = Object.fromEntries(LORD_KEYS.map((k) => [k, k === 'Caligo' ? [5, 6, 7, 8, 9] : [10, 10, 10, 10, 10]]));
  const everValue = (k: string) => (k === 'Gladius' ? [100, 100, 100, 100, 100] : k === 'Gnoster' ? [25, 33, 40, 50, 66] : [0, 0, 0, 0, 0]);
  return {
    level: kind === 'open' ? null : 5,
    nightlords,
    maps: { Standard: 65, TheCrater: 20 },
    everdark: Object.fromEntries(EVER_KEYS.map((k) => [k, everValue(k)])),
  };
}

export function createFakeWiki(q: URLSearchParams, record: (call: string, args?: unknown) => void): WikiApi {
  return {
    getWiki: async () => {
      record('getWiki');
      if (q.get('wikislow')) await new Promise((r) => window.setTimeout(r, SLOW_WIKI_MS));
      return view(q);
    },
    getWikiOdds: async () => {
      record('getWikiOdds');
      return odds(q);
    },
    // 探测靠查询串看选的是哪张图；none.webp 用来测缺图
    wikiImageUrl: (rel) => (rel === 'img/none.webp' ? asset('__none__.webp') : `${asset('sl_cursor.png')}?${rel}`),
  };
}
