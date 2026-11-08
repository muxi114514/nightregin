// 深夜与联机：强制深夜、层数、永夜王、解锁深夜、多人时敌人倍率。改动发给 Rust，停顿 300ms 后写回
import type { ConfigFileId, Field, GameSettings, LordsView, SettingChange } from '../../api';
import { confirm } from '../../ui/dialog';
import type { Desc } from '../../ui/desc';
import { asset } from '../../ui/dom';
import type { Row, Selectable, Value } from '../../ui/rows';
import { toastError } from '../../ui/toast';
import type { AppCtx } from '../app';
import type { SysHost, SysPage } from '../sys';
import { LORD_COUNT, LORDS, createLordsGrid, lordImage, lordWikiId } from './lords';

const FILE_NAME: Record<ConfigFileId, string> = {
  nighter: '深夜设置',
  nrsc: '无缝联机设置',
  chanceSetter: '夜王几率设置',
};
const NJ = 'DepthChange\\nighter.json';
const NR = 'SeamlessCoop\\nrsc_settings.ini';
const CS = 'dll\\ChanceSetter.ini';
const RUNNING_HINT = '游戏运行中，改动要重启游戏后生效。';
const ALL_LORDS = LORDS.map((_, i) => i);

export interface NightPage extends SysPage {
  /** Rust 发来的最新值（写回完成或磁盘被手改） */
  apply(v: GameSettings): void;
  /** 等设置读完再进永夜王子层：从图鉴直接过来时数据可能还没读 */
  openLords(focus: number): void;
}

function value<T>(f: Field<T> | undefined): Value<T> {
  if (f?.state === 'ok') return { ok: true, value: f.value };
  return { ok: false, message: f?.message ?? '读取失败' };
}

type ScalingKey = 'healthScaling' | 'damageScaling' | 'postureScaling';
/** 换档时的一档：名单，或整合包默认 */
type Stage = number[] | 'factory';

export function createNightPage(ctx: AppCtx): NightPage {
  let s: GameSettings | null = null;
  let host: SysHost | null = null;
  let failedFile: ConfigFileId | null = null;

  const lordsView = (): LordsView | null => (s?.lords.state === 'ok' ? s.lords.value : null);
  const lordList = (): number[] => lordsView()?.list ?? [];
  const grid = createLordsGrid({
    host: () => host as SysHost,
    list: lordList,
    onChange: setLords,
    onClose: () => host?.exitSub('lords'),
    openWiki: (i) => ctx.go({ page: 'wiki', id: lordWikiId(i), from: { page: 'lords', focus: i } }),
    dev: () => ctx.devMode(),
  });

  function send(change: SettingChange): void {
    ctx.api.setGameSetting(change).catch(toastError);
  }

  function setLords(list: number[]): void {
    if (!s) return;
    s.lords = { state: 'ok', value: { list, odds: 'picked', hasFactory: !!lordsView()?.hasFactory } };
    if (list.length > 0 && list.length < LORD_COUNT) s.lordPicks = list;
    send({ key: 'lords', value: list });
  }

  function setFactory(): void {
    if (!s) return;
    s.lords = { state: 'ok', value: { list: [], odds: 'factory', hasFactory: true } };
    send({ key: 'lordsFactory' });
  }

  /** 无 / 已挑选（有组合时）/ 全部 / 整合包默认（有出厂副本时）；手改的几率算在最后一档 */
  function stages(): Stage[] {
    const list = lordList();
    const picks = list.length > 0 && list.length < LORD_COUNT ? list : s?.lordPicks;
    return [[], ...(picks?.length ? [picks] : []), ALL_LORDS, ...(lordsView()?.hasFactory ? ['factory' as const] : [])];
  }

  function stageIndex(all: Stage[]): number {
    const v = lordsView();
    if (v?.odds !== 'picked') return all.length - 1;
    const n = v.list.length;
    return n === 0 ? 0 : n === LORD_COUNT ? all.indexOf(ALL_LORDS) : 1;
  }

  function withRunning(d: Desc): Desc {
    return s?.gameRunning ? { ...d, paragraphs: [...d.paragraphs, RUNNING_HINT] } : d;
  }

  function failure(f: Field<unknown> | undefined): string[] {
    return f?.state === 'failed' ? [`读取失败：${f.message}`] : [];
  }

  function scaling(id: ScalingKey, label: string, ini: string): Row {
    return {
      id, label, type: 'number', min: 10, max: 300, step: 10, unit: '%',
      get: () => value(s?.[id]),
      set: (v) => {
        if (!s) return;
        s[id] = { state: 'ok', value: v };
        send({ key: id, value: v });
      },
      desc: () => withRunning({
        img: asset('fl_dragon.png'),
        meta: '多人加成',
        paragraphs: [...failure(s?.[id]), '多人时敌人属性的倍率，100% 为默认。无缝联机最多 6 人同行，人数由模组决定，不需要设置。'],
        dev: [NR, `[SCALING] ${ini}`],
      }),
    };
  }

  function toggle(id: 'forceDeepNight' | 'unlockDeepNight', label: string, img: string, text: string, key: string): Row {
    return {
      id, label, type: 'toggle',
      get: () => value(s?.[id]),
      set: (v) => {
        if (!s) return;
        s[id] = { state: 'ok', value: v };
        send({ key: id, value: v });
      },
      desc: () => withRunning({
        img: asset(img),
        meta: '深夜',
        paragraphs: [...failure(s?.[id]), text],
        dev: [NJ, key],
      }),
    };
  }

  const forceOff = () => s?.forceDeepNight.state === 'ok' && !s.forceDeepNight.value;

  function lordsText(): string {
    const v = lordsView();
    if (v?.odds === 'factory') return '整合包默认';
    if (v?.odds === 'custom') return '自定义几率';
    const n = lordList().length;
    return n === 0 ? '无' : n === LORD_COUNT ? '全部' : `已挑选 ${n} 位`;
  }

  const rows = (): Row[] => (!s ? [] : [
    toggle('forceDeepNight', '强制深夜', 'fl_veil.png', '开启后，每次出征都是深夜。关闭后照常在出征前自己选择。', 'forceDeepNight.enable'),
    {
      id: 'deepNightLevel', label: '深夜层数', type: 'number', min: 1, max: 5, step: 1, unit: ' 层', off: forceOff,
      get: () => value(s?.deepNightLevel),
      set: (v) => {
        if (!s) return;
        s.deepNightLevel = { state: 'ok', value: v };
        send({ key: 'deepNightLevel', value: v });
      },
      desc: () => withRunning({
        img: asset(`lv_${s?.deepNightLevel.state === 'ok' ? Math.min(5, Math.max(1, s.deepNightLevel.value)) : 5}.png`),
        meta: '深夜',
        paragraphs: [
          ...failure(s?.deepNightLevel),
          '强制深夜时的层数，1～5 层。整合包的首领数值按深夜 5 层调整。',
          ...(forceOff() ? ['强制深夜关闭时，这一项不生效。'] : []),
        ],
        dev: [NJ, 'forceDeepNight.level'],
      }),
    },
    {
      id: 'lords', label: '永夜王', type: 'action',
      value: lordsText,
      enter: () => '挑选',
      failed: () => (s?.lords.state === 'failed' ? s.lords.message : null),
      run: () => grid.open(),
      step: (dir) => {
        const all = stages();
        const next = Math.max(0, Math.min(all.length - 1, stageIndex(all) + dir));
        if (next === stageIndex(all)) return;
        const stage = all[next];
        if (stage === 'factory') setFactory();
        else setLords([...stage]);
      },
      atEnd: () => {
        const all = stages();
        return { lo: stageIndex(all) === 0, hi: stageIndex(all) === all.length - 1 };
      },
      desc: () => withRunning({
        img: lordImage(lordList()[0] ?? 0, lordList().length > 0),
        meta: '深夜',
        paragraphs: [
          ...failure(s?.lords),
          '选出以永夜王形态出现的夜王，比普通形态更强。按 Enter 或单击逐个挑选；← → 在「无 / 已挑选 / 全部 / 整合包默认」之间切换。',
          ...(lordsView()?.odds === 'factory' ? ['整合包默认：深夜 1～5 层依次有 25 / 33 / 40 / 50 / 66% 的几率成为永夜版。'] : []),
        ],
        strip: LORDS.map((_, i) => ({ src: lordImage(i, lordList().includes(i)), on: lordList().includes(i) })),
        dev: [CS, '[everdarkChances.depth1～5]'],
      }),
    },
    toggle('unlockDeepNight', '解锁深夜', 'fl_veil.png', '没有解锁深夜的存档，也能选择深夜。', 'forceUnlockDeepNight'),
    { type: 'separator', id: 'sep' },
    scaling('healthScaling', '敌人血量', 'health_scaling'),
    scaling('damageScaling', '敌人伤害', 'damage_scaling'),
    scaling('postureScaling', '敌人架势', 'posture_scaling'),
  ]);

  function fileOf(r: Selectable): ConfigFileId | null {
    const f = s?.[r.id as keyof GameSettings];
    return f && typeof f === 'object' && 'state' in f && f.state === 'failed' ? f.file : null;
  }

  async function restore(r: Selectable): Promise<void> {
    failedFile = fileOf(r);
    if (!failedFile) return;
    const file = failedFile;
    const ok = await confirm({
      text: `把${FILE_NAME[file]}恢复成整合包出厂设置？`,
      ok: '恢复默认',
      cancel: '取消',
      destructive: true,
    });
    if (!ok) return;
    try {
      s = await ctx.api.restoreConfigDefault(file);
      host?.reload();
    } catch (e) {
      toastError(e);
    }
  }

  return {
    defaultRow: 'forceDeepNight',
    rows,
    attach(h) {
      host = h;
      h.grid.append(grid.el);
    },
    async load() {
      s = await ctx.api.getGameSettings();
    },
    apply(v) {
      s = v;
      host?.refresh();
    },
    openLords(focus) {
      ctx.api.getGameSettings()
        .then((v) => {
          s = v;
          grid.open(focus);
        })
        .catch(toastError);
    },
    onFailed: (r) => void restore(r),
    handleKey: (k) => (grid && isGridKey() ? grid.handleKey(k) : false),
  };

  function isGridKey(): boolean {
    return !!host?.grid && !host.grid.hidden;
  }
}
