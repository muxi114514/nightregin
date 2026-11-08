"""图鉴数据修正：展示文字与开发备注分开（note 只留在数据里，不上界面）；手写内容标 h / srcH；
「出现」按整合包的 ChanceSetter.ini 生成；补大空洞；去掉自编的夜王叫法。
用法：python -I wiki_fix.py <wiki_sample.json> <ChanceSetter.ini> <输出 wiki_sample.js>"""
import configparser, json, re, sys

src, ini, out = sys.argv[1:4]
D = json.load(open(src, encoding='utf-8'))
cs = configparser.ConfigParser(strict=False, inline_comment_prefixes=(';',))
cs.optionxform = str
cs.read_string('[top]\n' + open(ini, encoding='utf-8').read())
NL = ['Gladius', 'Adel', 'Gnoster', 'Maris', 'Libra', 'Fulghor', 'Caligo', 'Heolstor', 'Harmonia', 'Straghess']
MAPS = {'map-crater': 'TheCrater', 'map-mountaintop': 'Mountaintop', 'map-rotted': 'RottedWoods',
        'map-noklateo': 'ShroudedCity', 'map-hollow': 'GreatHollow'}
on = lambda s: cs.getboolean(s, 'enabled', fallback=False)


def depths(sec, name):
    v = [cs.getint(f'{sec}.depth{d}', name, fallback=0) for d in range(1, 6)]
    return f'{v[0]}%' if len(set(v)) == 1 else '深夜 1～5 层 ' + ' / '.join(map(str, v)) + '%'


def drought():
    s = 'shiftingEarthDroughtProtection'
    if not on(s):
        return None
    return f'连续 {cs.getint(s, "startAfterStandardStreak")} 次普通地图后，特异地形合计几率每次 +{cs.getint(s, "extraShiftingEarthChancePerStandard")}%'


def move_note(e, note):
    e['note'] = '；'.join(filter(None, [e.get('note'), note]))


for c in D['categories']:
    for e in c.get('entries', []):
        s = e.get('source') or ''
        m = re.search(r'权重 (\d+)/1000', s)
        if m:   # 依据掉落脚本推算的概率：界面只写玩家能懂的说法
            move_note(e, s)
            e['source'] = f'首领掉落球 · 第三选项 · 约 {round(int(m.group(1)) / 10)}%'
        elif s.startswith('（占位）') and '（' in s[4:]:
            move_note(e, s)
            e['source'] = '（占位）获取方式待补'
        for st in e.get('stats', []):
            if re.search(r'[a-z_]+\.[a-z_]+', st['v']):   # 数值里夹着配置键名
                move_note(e, st['v'])
                st['v'] = re.sub(r'（[^）]*[a-z_]+\.[a-z_]+[^）]*）', '（可在「模组设置」里调整）', st['v'])
        if c['key'] == 'mechanic' and 'DLL' in s:
            move_note(e, s)
            e['source'] = '整合包自带模组'

    if c['key'] == 'talisman':
        for e in c['entries']:
            if e['id'] == 'talisman-20015':   # 两条效果合并，免得读成 18% 再加 18%
                move_note(e, '效果 1 / 2 合并为一条；游戏内文本待同步修改')
                e['stats'] = [{'k': '效果', 'v': '“多才多艺”常驻 9 层（共提升 18% 攻击力）'},
                              {'k': '状态栏', 'v': '多才多艺 1 层：提升 2% 攻击力'}]

    if c['key'] == 'map':
        hollow = {'id': 'map-hollow', 'name': '大空洞', 'sub': '特异地形', 'icon': '', 'image': '', 'desc': '简介待补。', 'lore': '',
                  'stats': [], 'source': '', 'isNew': False, 'tag': 'DLC', 'real': {'name': True, 'desc': False, 'icon': False}}
        if not any(e['id'] == 'map-hollow' for e in c['entries']):
            c['entries'].insert(4, hollow)
        for e in c['entries']:
            if e['id'] in MAPS and on('mapChances'):
                move_note(e, '出现 / 消失两行原为手写，已按 ChanceSetter.ini [mapChances] 重写')
                e['source'] = f'每次出征 {cs.getint("mapChances", MAPS[e["id"]])}%（普通地图 {cs.getint("mapChances", "Standard")}%）'
                d = drought()
                e['stats'] = [{'k': '保底', 'v': d}] if d else []
            elif e['id'] in ('map-lake', 'map-forge'):
                move_note(e, '地图图标 = ' + '、'.join(s['v'] for s in e.get('stats', [])))
                e['stats'] = []
                e['srcH'] = True

    if c['key'] == 'enemy':
        for e in c['entries']:
            i = int(e['id'].split('-')[1])
            e['group'] = '夜王' if i < 7 else '黑夜王' if i == 7 else 'DLC 夜王'
            if i >= 8:   # 「女武神」「尸鬼」是自编叫法，去掉
                e['sub'] = e['sub'].split(' · ')[0]
            e['stats'] = [s for s in e.get('stats', []) if not s['k'].startswith('nighter')]
            for s in e['stats']:
                if s['k'] == '有利属性':
                    s['h'] = 1   # 属性映射是推断的
            if on('nightlordChances'):
                move_note(e, '出现几率按 ChanceSetter.ini [nightlordChances] 生成')
                e['source'] = f'每次出征 {depths("nightlordChances", NL[i])}（按整合包几率设定）'
                if i == 8 and on('everdarkChances'):
                    e['stats'].append({'k': '永夜形态', 'v': depths('everdarkChances', NL[i])})

open(out, 'w', encoding='utf-8').write('window.WIKI = ' + json.dumps(D, ensure_ascii=False, indent=1) + ';\n')
open(out + '.check.txt', 'w', encoding='utf-8').write('\n'.join(
    f"{e['id']} | {e.get('sub')} | {e.get('source')} | {[(s['k'], s['v']) for s in e.get('stats', [])]}"
    for c in D['categories'] for e in c.get('entries', [])))
print('ok')
