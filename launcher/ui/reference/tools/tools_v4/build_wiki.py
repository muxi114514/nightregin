"""图鉴导出第四步：从 text.json / params.json / 菜单图集组装图鉴样本（只读游戏文件）。
用法：python -I build_wiki.py <wiki_work 目录> <wiki_data 输出目录> <GA 美术 png 目录>
产物：wiki_sample.json、wiki_sample.js（window.WIKI = ...;）、img\\*.webp
real 里 name / desc / icon（及 source）为 true 表示取自游戏真实数据，false 表示手写或占位。
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from menu_sprites import Sprites  # noqa: E402
from PIL import Image  # noqa: E402

WORK, OUT, GA_ART = sys.argv[1], sys.argv[2], sys.argv[3]
IMG = os.path.join(OUT, 'img')
os.makedirs(IMG, exist_ok=True)
sys.stdout.reconfigure(encoding='utf-8')
T = json.load(open(os.path.join(WORK, 'text.json'), encoding='utf-8'))
P = json.load(open(os.path.join(WORK, 'params.json'), encoding='utf-8'))
Z = T['zhocn']
SP = Sprites(os.path.join(WORK, 'menu'))
PLACEHOLDER = '（占位）'
issues = []


def tx(fmg, i):
    """先查本体 FMG，再查 _dlc01"""
    for key in (fmg, fmg + '_dlc01'):
        v = Z.get(key, {}).get(str(i))
        if v and v.strip() and v.strip() != '%null%':
            return v.strip()
    return None


def rows(table):
    return {r['ID']: r for r in P['pkg'][table]}


def save_sprite(sprite, out_name, size=None, quality=86):
    img = SP.image(sprite)
    if img is None:
        issues.append(f'图片缺失：{sprite}')
        return ''
    if size:
        img = img.copy()
        img.thumbnail(size, Image.LANCZOS)
    img.save(os.path.join(IMG, out_name), 'WEBP', quality=quality, method=6)
    return 'img/' + out_name


def save_file(path, out_name, size):
    if not os.path.exists(path):
        issues.append(f'图片缺失：{path}')
        return ''
    img = Image.open(path).convert('RGBA')
    img.thumbnail(size, Image.LANCZOS)
    img.save(os.path.join(IMG, out_name), 'WEBP', quality=86, method=6)
    return 'img/' + out_name


NEW_195 = {t: set(r['ID'] for r in P['pkg'][t]) - set(P['ids']['rel194'][t]) for t in ('EquipParamWeapon', 'EquipParamAccessory', 'Magic')}
VANILLA = {t: set(P['ids']['vanilla'][t]) for t in ('EquipParamWeapon', 'EquipParamAccessory', 'Magic')}


def origin(table, i):
    if i in NEW_195[table]:
        return 'GA 移植'
    return '原版' if i in VANILLA[table] else 'MMV 新增'


# ---------------- 武器 ----------------
# wepType → CL_MenuText 类别名（60000 段；90 以后比法环错后 2 位，按 MMV 新武器的实际类别对出来的）
WEP_CAT = {1: 60010, 3: 60015, 5: 60020, 7: 60025, 9: 60030, 11: 60035, 13: 60040, 14: 60043, 15: 60045, 16: 60046, 17: 60050,
           19: 60055, 21: 60060, 23: 60065, 24: 60067, 25: 60070, 28: 60077, 29: 60080, 31: 60085, 35: 60090, 37: 60100, 39: 60155,
           41: 60102, 50: 60104, 51: 60105, 53: 60110, 55: 60115, 56: 60117, 57: 60120, 61: 60130, 65: 60140, 67: 60145, 69: 60150,
           87: 60175, 90: 60180, 91: 60181, 92: 60182, 93: 60183, 94: 60184, 95: 60185, 96: 60186, 97: 60187}
ATK = [('attackBasePhysics', '物理'), ('attackBaseMagic', '魔力'), ('attackBaseFire', '火'), ('attackBaseThunder', '雷'), ('attackBaseDark', '圣')]
COR = [('correctStrength', '力'), ('correctAgility', '技'), ('correctMagic', '智'), ('correctFaith', '信'), ('correctLuck', '感')]
# 武器掉落：tools\ga_loot.ps1 放进首领球第三选项（与徘徊赐福同池，千分比权重），未实测
GA_LOOT = {89000000: 32, 20008: 32, 20015: 31}
# 格拉狄乌斯的大剑 4700000 的说明在中文包里还是英文（MMV 未翻译），样本换成黑骑士大剑
WEAPONS = [89000000, 14530000, 2160000, 4600000, 10180000, 4500000, 6770000, 6660000, 4130000, 9060000, 4060000, 2140000]


def grade(v):
    if v <= 0:
        return None
    for lim, g in ((175, 'S'), (140, 'A'), (90, 'B'), (60, 'C'), (25, 'D')):
        if v >= lim:
            return g
    return 'E'


def loot_source(i):
    if i in GA_LOOT:
        return f'首领掉落球第三选项（与徘徊赐福同池，权重 {GA_LOOT[i]}/1000，依据 ga_loot.ps1，未实测）', True
    return PLACEHOLDER + '获取方式待补（掉落表未解析）', False


def weapon_entries():
    W, SA = rows('EquipParamWeapon'), rows('SwordArtsParam')
    out = []
    for i in WEAPONS:
        r = W[i]
        name = tx('item/WeaponName', i)
        info, cap = tx('item/WeaponInfo', i), tx('item/WeaponCaption', i)
        cat = Z['menu/CL_MenuText'].get(str(WEP_CAT.get(r['wepType'], -1)))
        stats = [{'k': '类别', 'v': cat or f'wepType {r["wepType"]}'}]
        atk = [f'{n} {r[f]}' for f, n in ATK if r[f] > 0]
        stats.append({'k': '基础攻击力', 'v': ' / '.join(atk) or '—'})
        cor = [f'{n} {grade(r[f])}' for f, n in COR if grade(r[f])]
        stats.append({'k': '属性补正', 'v': ' / '.join(cor) or '—'})
        art = SA.get(r['swordArtsParamId'])
        art_name = art and tx('item/ArtsName', art['textId'])
        stats.append({'k': '战技', 'v': art_name or PLACEHOLDER})
        src, src_real = loot_source(i)
        desc = info or (cap.split('\n')[0] if cap else None)
        out.append({
            'id': f'weapon-{i}', 'name': name or PLACEHOLDER, 'sub': (cat or '') + ' · ' + origin('EquipParamWeapon', i),
            'icon': save_sprite(f'MENU_ItemIcon_{r["iconId"]:05d}', f'w_{i}.webp'), 'image': '',
            'desc': desc or PLACEHOLDER, 'lore': cap or '', 'stats': stats, 'source': src,
            'isNew': i in NEW_195['EquipParamWeapon'], 'tag': origin('EquipParamWeapon', i),
            'real': {'name': bool(name), 'desc': bool(desc), 'icon': SP.where(f'MENU_ItemIcon_{r["iconId"]:05d}') is not None, 'source': src_real},
            'descFrom': 'WeaponInfo' if info else 'WeaponCaption 第一行',
            'ref': {'table': 'EquipParamWeapon', 'row': i, 'iconId': r['iconId'], 'swordArtsParamId': r['swordArtsParamId']},
        })
    return out


# ---------------- 护符 ----------------
TALISMANS = [20008, 20015, 8050, 8110, 1210, 3090]


def find_text(word, prefix='menu/'):
    for key, d in Z.items():
        if not key.startswith(prefix):
            continue
        for k, v in d.items():
            if word in v and len(v) > len(word) + 4:
                return key, int(k), v
    return None


def talisman_entries():
    A, AE = rows('EquipParamAccessory'), rows('AttachEffectParam')
    out = []
    for i in TALISMANS:
        r = A[i]
        name, info, cap = tx('item/AccessoryName', i), tx('item/AccessoryInfo', i), tx('item/AccessoryCaption', i)
        stats = []
        for n, s in enumerate(('spEffectId_1', 'spEffectId_2', 'spEffectId_3'), 1):
            a = AE.get(r[s])
            if not a:
                continue
            t = tx('item/AttachEffectName', a['attachTextId'])
            stats.append({'k': f'效果 {n}', 'v': t or PLACEHOLDER})
        if i == 20015:
            hit = find_text('多才多艺')
            if hit:
                stats.append({'k': '多才多艺', 'v': hit[2].replace('\n', ' ')[:80]})
        src, src_real = loot_source(i)
        icon_name = f'MENU_ItemIcon_{r["iconId"]:05d}'
        out.append({
            'id': f'talisman-{i}', 'name': name or PLACEHOLDER, 'sub': '护符 · ' + origin('EquipParamAccessory', i),
            'icon': save_sprite(icon_name, f't_{i}.webp'), 'image': '',
            'desc': info or PLACEHOLDER, 'lore': cap or '', 'stats': stats, 'source': src,
            'isNew': i in NEW_195['EquipParamAccessory'], 'tag': origin('EquipParamAccessory', i),
            'real': {'name': bool(name), 'desc': bool(info), 'icon': SP.where(icon_name) is not None, 'source': src_real},
            'ref': {'table': 'EquipParamAccessory', 'row': i, 'iconId': r['iconId']},
        })
    return out


# ---------------- 敌人（夜王） ----------------
# NightBossMenuParam：0~6 七位夜王（= nighter.json 的 superNightLordList 编号），7 黑夜王，8/9 DLC，10~16 = 0~6 的永夜之王形态
AFFINITY = {2: '火', 3: '雷', 4: '圣', 5: '毒', 11: '发狂'}  # 由各夜王说明文本与原版弱点对出来，10 未确认
NICK = {0: '三狼', 1: '大嘴', 2: '蝴蝶与蝎子', 3: '水母', 4: '山羊', 5: '人马', 6: '冰龙', 7: '黑夜王', 8: '女武神', 9: '尸鬼'}
NPC_NAME = {0: 907500000, 1: 907510000, 2: 907520000, 3: 907540000, 4: 907550000, 5: 907610000, 6: 907620000, 7: 907580001, 8: 907640000, 9: 907630000}
HAND = {  # 手写简介（标记 real.desc = false）
    0: '多头魔狼，战斗中会分化成数只影狼围猎，火焰缠身。',
    1: '巨颚黑兽，吐雷与扑咬吞噬，体型庞大压迫感强。',
    2: '空中的蛾与地面的蝎双体作战，魔光弹幕与寄生蛾毒。',
    3: '漂浮的水母状存在，把战场化作海洋，攻击常从视野外袭来。',
    4: '模仿人类的山羊头恶魔，以虚假黄金交易诱人发狂。',
    5: '古老人马骑士，虽属黑夜仍能施展神圣之力，冲锋迅猛。',
    6: '雾中史前冰龙，寒雾与冰霜吐息会迅速夺走体温。',
    7: '黑夜的起源，打倒七位夜王后才会现身的最终首领。',
    8: '七位身披白甲的女武神（DLC），手持英雄武器。',
    9: '由尸骸与残骸拼成的尸鬼（DLC），没有真正的弱点。',
}


def enemy_entries():
    NB = rows('NightBossMenuParam')
    label = Z['menu/CL_MenuText'].get('131040')  # 永夜之王
    out = []
    for idx in range(10):
        r = NB[idx]
        name = tx('menu/CL_MenuText', r['bossNameId'])
        exp = tx('menu/CL_MenuText', r['expeditionNameId'])
        lore = tx('menu/CL_MenuText', r['descriptionId'])
        title = tx('item/NpcName', NPC_NAME[idx])
        stats = [{'k': '远征', 'v': exp or PLACEHOLDER}, {'k': '称号', 'v': title or PLACEHOLDER}]
        aff = AFFINITY.get(r['effectiveAffinity1'])
        stats.append({'k': '有利属性', 'v': aff or '—'})
        entry = {
            'id': f'nightlord-{idx}', 'name': name or PLACEHOLDER, 'sub': f'{exp} · {NICK[idx]}',
            'icon': save_sprite(f'MENU_MenuIcon_{r["largeBossIconId"]}', f'nl_{idx}.webp', (320, 320)),
            'image': save_sprite(f'MENU_MenuIcon_{r["expeditionBackgroundId"]}', f'nl_{idx}_bg.webp', (816, 360), 80),
            'desc': HAND[idx], 'lore': lore or '', 'stats': stats,
            'source': '最终远征（打倒七位夜王后）' if idx == 7 else ('DLC 远征' if idx >= 8 else '远征目标（第三夜）'),
            'isNew': False, 'tag': '原版',
            'real': {'name': bool(name), 'desc': False, 'icon': True, 'lore': bool(lore)},
            'ref': {'table': 'NightBossMenuParam', 'row': idx, 'bossNameId': r['bossNameId'], 'largeBossIconId': r['largeBossIconId']},
        }
        ever = NB.get(idx + 10)
        if idx <= 6 and ever:
            entry['nighterIndex'] = idx
            entry['everdark'] = {
                'name': f'{label} {name}', 'label': label, 'nameNote': '游戏里没有单独的永夜王名字：远征卡片在原名上方加「永夜之王」（CL_MenuText 131040），这里按此拼接',
                'icon': save_sprite(f'MENU_MenuIcon_{ever["largeBossIconId"]}', f'nl_{idx}_ever.webp', (320, 320)),
                'depthWeights': [ever[f'depth{d}ChanceWeight'] for d in range(1, 6)],
            }
            stats.append({'k': '永夜王形态', 'v': entry['everdark']['name']})
            stats.append({'k': 'nighter 编号', 'v': str(idx)})
        elif idx == 8 and NB.get(18):
            entry['everdark'] = {'name': f'{Z["menu/CL_MenuText_dlc01"].get("131042", "")} {name}', 'label': Z['menu/CL_MenuText_dlc01'].get('131042', ''),
                                 'icon': save_sprite(f'MENU_MenuIcon_{NB[18]["largeBossIconId"]}', f'nl_{idx}_ever.webp', (320, 320))}
        out.append(entry)
    return out


# ---------------- 大地图 ----------------
def map_entries():
    out = []
    shifts = [('crater', 12, 403430, 26101021), ('mountaintop', 11, 403420, 26101022), ('rotted', 13, 403400, 26101023), ('noklateo', 15, 403410, 26101024)]
    RM = rows('RareMapInfoMenuParam')
    for key, rid, tut, talk in shifts:
        full = tx('menu/CL_MenuText', RM[rid]['nameId']) or ''
        name = full.split('：', 1)[-1]
        body = tx('menu/TutorialBody', tut)
        ask = tx('menu/EventTextForTalk', talk) or ''
        cost = ask.split('为王之证的所需数量：')[-1].split('\n')[0] if '为王之证' in ask else None
        out.append({
            'id': f'map-{key}', 'name': name or PLACEHOLDER, 'sub': '特异地形',
            'icon': '', 'image': '', 'desc': body.split('\n')[0] if body else PLACEHOLDER, 'lore': body or '',
            'stats': [{'k': '引发', 'v': f'圆桌厅堂消耗为王之证 ×{cost}' if cost else PLACEHOLDER}, {'k': '消失', 'v': '数次出击或在圆桌厅堂消磨时间后'}],
            'source': '打倒夜王等行动后随机出现，也可在圆桌厅堂主动引发', 'isNew': False, 'tag': '原版',
            'real': {'name': bool(name), 'desc': bool(body), 'icon': False},
            'ref': {'table': 'RareMapInfoMenuParam', 'row': rid, 'tutorialText': tut},
        })
    body = tx('menu/TutorialBody', 403240) or ''
    parts = {'湖沼': '', '锻造村': ''}
    for chunk in body.split('<?image@'):
        for k in parts:
            if f'：{k}' in chunk:
                parts[k] = chunk.split(f'：{k}', 1)[1].strip().replace('\n', '')
    for key, word, icon in (('lake', '湖沼', 42632), ('forge', '锻造村', 42633)):
        out.append({
            'id': f'map-{key}', 'name': word, 'sub': '可攻略地点',
            'icon': save_sprite(f'MENU_MenuIcon_{icon}', f'map_{key}.webp'), 'image': '',
            'desc': parts[word] or PLACEHOLDER, 'lore': '', 'stats': [{'k': '地图图标', 'v': f'菜单图标 {icon}'}],
            'source': '地图上的可攻略据点（原为 DLC 地图点，整合包补进基础图集）', 'isNew': False, 'tag': 'MMV',
            'real': {'name': True, 'desc': bool(parts[word]), 'icon': True},
            'ref': {'text': 'TutorialBody_dlc01 403240'},
        })
    return out


# ---------------- 机制（手写，依据 nr_stat_alloc.toml 注释） ----------------
def mech_entries():
    # GA 美术里 Grace_LevelUp / AshOfWar 等子图是全透明的空槽，改用游戏道具图标示意（徘徊赐福 680、锻造石【１】~【３】 2003/2007/2008）
    def ga(icon, out_name):
        return save_sprite(f'MENU_ItemIcon_{icon:05d}', out_name) if icon else ""
    data = [
        ('stat-alloc', '属性分配', '自选每项属性的成长', 0,
         '在遗物装配界面打开「属性分配」面板，按方案逐级分配各项属性，方案随器皿保存。',
         [('打开', 'F6 或手柄 L3（遗物装配界面）'), ('单项范围', '1 ~ 70'), ('联机', '效果同步给队友')]),
        ('passive-upgrade', '词条同步升档', '锻造石强化时词条跟着升', 2007,
         '用锻造石强化武器时，武器上的词条同步升到同类的上位档，数值全部取自原版参数表。',
         [('触发', '锻造台使用锻造石强化'), ('数值来源', '原版参数表的上位档')]),
        ('passive-inherit', '继承词条', '锻造台把词条整把转移', 2003,
         '锻造台的「继承词条」：把一把武器的全部词条整把覆盖到另一把上，来源武器随之消失。',
         [('规则', '整把覆盖，不能挑单条'), ('档位限制', '来源稀有度不能低于目标（白 / 蓝 / 紫红金）')]),
        ('tiered-buffs', '金紫蓝三档锻体', '首领掉落球获得的永久增益', 680,
         '首领掉落球可选锻体，分金、紫、蓝三档；部分锻体为角色专属，本局内永久生效。',
         [('获取', '首领掉落球'), ('档位', '金 / 紫 / 蓝'), ('专属', '部分锻体仅限对应角色')]),
        ('boss-loot', '首领掉落三选一', '击败首领后三个选项任选其一', 2008,
         '击败首领后掉落三选一；第三个选项（局内词条 / 徘徊赐福）有概率换成锻造石【３】。',
         [('锻造石【３】概率', '10%（boss_loot.smithing_stone_percent）'), ('其他', '第三选项另有锻体与移植物品')]),
    ]
    out = []
    for key, name, sub, icon, desc, stats in data:
        out.append({
            'id': f'mech-{key}', 'name': name, 'sub': sub, 'icon': ga(icon, f'mech_{key}.webp'), 'image': '',
            'desc': desc, 'lore': '', 'stats': [{'k': k, 'v': v} for k, v in stats],
            'source': '整合包 DLL（nr_stat_alloc）', 'isNew': False, 'tag': '整合包',
            'real': {'name': False, 'desc': False, 'icon': False}, 'note': '手写，依据 nr_stat_alloc.toml 注释与开发记录；图标借用游戏道具图标示意，属性分配暂无图标',
        })
    return out


def main():
    cats = [
        {'key': 'weapon', 'name': '武器', 'entries': weapon_entries()},
        {'key': 'talisman', 'name': '护符', 'entries': talisman_entries()},
        {'key': 'spell', 'name': '法术', 'entries': [],
         'note': '整合包没有移植 GA 法术（Magic 表与 1.9.4 相比无新增行）；MMV 自带 48 个新增法术行，需要时可改用'},
        {'key': 'map', 'name': '大地图', 'entries': map_entries()},
        {'key': 'enemy', 'name': '敌人', 'entries': enemy_entries()},
        {'key': 'mechanic', 'name': '机制', 'entries': mech_entries()},
    ]
    data = {'version': '1.9.5', 'generatedBy': 'tools_v4/build_wiki.py', 'categories': cats}
    text = json.dumps(data, ensure_ascii=False, indent=1)
    open(os.path.join(OUT, 'wiki_sample.json'), 'w', encoding='utf-8').write(text)
    open(os.path.join(OUT, 'wiki_sample.js'), 'w', encoding='utf-8').write('window.WIKI = ' + text + ';\n')
    for c in cats:
        print(c['key'], len(c['entries']), [e['name'] for e in c['entries']])
    total = sum(os.path.getsize(os.path.join(IMG, f)) for f in os.listdir(IMG))
    print('img bytes', total, 'files', len(os.listdir(IMG)))
    for i in issues:
        print('ISSUE', i)


if __name__ == '__main__':
    main()
