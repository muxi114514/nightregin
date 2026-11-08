"""图鉴分类：敌人（夜王）、大地图。每个函数返回 (条目列表, 分类的筛选项)。"""
from wiki_lib import PORTRAIT_SIZE

# NightBossMenuParam：0~6 七位夜王，7 黑夜王，8/9 DLC，10~16 是 0~6 的永夜形态，18 是 8 的永夜形态
CHANCE_KEYS = ['Gladius', 'Adel', 'Gnoster', 'Maris', 'Libra', 'Fulghor', 'Caligo', 'Heolstor', 'Harmonia', 'Straghess']
# 行号 → 永夜王网格下标
LORD_INDEX = {0: 0, 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 8: 7}
# 由各夜王说明文本与原版弱点对出来，10 未确认
AFFINITY = {2: '火', 3: '雷', 4: '圣', 5: '毒', 11: '发狂'}
NICK = {0: '三狼', 1: '大嘴', 2: '蝴蝶与蝎子', 3: '水母', 4: '山羊', 5: '人马', 6: '冰龙', 7: '黑夜王'}
NPC_NAME = {0: 907500000, 1: 907510000, 2: 907520000, 3: 907540000, 4: 907550000, 5: 907610000, 6: 907620000, 7: 907580001, 8: 907640000, 9: 907630000}
EVERDARK_LABEL_TEXT = 131040
EVERDARK_LABEL_DLC_TEXT = 131042

ENEMY_FILTERS = ['夜王', '黑夜王', 'DLC 夜王']
# 键, RareMapInfoMenuParam 行, TutorialBody, 出价文本, ChanceSetter 键, art 图
SHIFTS = [('crater', 12, 403430, 26101021, 'TheCrater', 'wm_volcano.png'),
          ('mountaintop', 11, 403420, 26101022, 'Mountaintop', 'wm_snowup.png'),
          ('rotted', 13, 403400, 26101023, 'RottedWoods', 'wm_rotten.png'),
          ('noklateo', 15, 403410, 26101024, 'ShroudedCity', 'wm_nokron.png')]
SHIFT_FILTER = '特异地形'
SITE_FILTER = '可攻略地点'
SITES = [('lake', '湖沼', 42632), ('forge', '锻造村', 42633)]
SITE_BODY = 403240
SHIFT_SOURCE = '打倒夜王等行动后随机出现，也可在圆桌厅堂主动引发'
SITE_SOURCE = '地图上的可攻略据点'


def lord_filter(row):
    return ENEMY_FILTERS[0] if row < 7 else ENEMY_FILTERS[1] if row == 7 else ENEMY_FILTERS[2]


def everdark(ctx, row, name, rows):
    """永夜版的头像和名字：游戏里没有单独的名字，只在原名上方加「永夜之王」"""
    ever = rows.get(row + 10)
    if row not in LORD_INDEX or not ever:
        return {}
    label = ctx.text('menu/CL_MenuText', EVERDARK_LABEL_DLC_TEXT if row == 8 else EVERDARK_LABEL_TEXT)
    return {
        'lord': LORD_INDEX[row],
        'everIcon': ctx.sprite(f'MENU_MenuIcon_{ever["largeBossIconId"]}', f'nl_{row}_ever.webp', PORTRAIT_SIZE),
        'everName': f'{label} {name}' if label else name,
    }


def enemies(ctx):
    rows = ctx.rows('NightBossMenuParam')
    out = []
    for row in range(10):
        r = rows[row]
        name = ctx.text('menu/CL_MenuText', r['bossNameId'])
        expedition = ctx.text('menu/CL_MenuText', r['expeditionNameId'])
        title = ctx.text('item/NpcName', NPC_NAME[row])
        affinity = AFFINITY.get(r['effectiveAffinity1'])
        stats = [{'k': '远征', 'v': expedition}, {'k': '称号', 'v': title}]
        stats = [s for s in stats if s['v']]
        if affinity:
            stats.append({'k': '有利属性', 'v': affinity, 'h': True})
        nick = NICK.get(row)
        out.append({
            'id': f'nightlord-{row}', 'name': name, 'sub': ' · '.join(x for x in (expedition, nick) if x),
            'icon': ctx.sprite(f'MENU_MenuIcon_{r["largeBossIconId"]}', f'nl_{row}.webp', PORTRAIT_SIZE),
            'portrait': True, 'filters': [lord_filter(row)], 'stats': stats,
            'lore': ctx.text('menu/CL_MenuText', r['descriptionId']),
            'chance': {'table': 'nightlord', 'key': CHANCE_KEYS[row]},
            **everdark(ctx, row, name, rows),
        })
    return out, ENEMY_FILTERS


def summon_cost(ctx, talk_id):
    """圆桌厅堂引发地变的价格写在对话文本里"""
    ask = ctx.text('menu/EventTextForTalk', talk_id) or ''
    return ask.split('为王之证的所需数量：')[-1].split('\n')[0].strip() if '为王之证' in ask else None


def site_texts(ctx):
    """湖沼和锻造村的说明写在同一段教程文本里，用图片标记分段"""
    body = ctx.text('menu/TutorialBody', SITE_BODY) or ''
    parts = {}
    for chunk in body.split('<?image@'):
        for _, word, _ in SITES:
            if f'：{word}' in chunk:
                parts[word] = chunk.split(f'：{word}', 1)[1].strip().replace('\n', '')
    return parts


def maps(ctx):
    rows = ctx.rows('RareMapInfoMenuParam')
    out = []
    for key, row, tutorial, talk, chance, art in SHIFTS:
        full = ctx.text('menu/CL_MenuText', rows[row]['nameId']) or ''
        body = ctx.text('menu/TutorialBody', tutorial)
        cost = summon_cost(ctx, talk)
        out.append({
            'id': f'map-{key}', 'name': full.split('：', 1)[-1], 'sub': SHIFT_FILTER,
            'icon': ctx.art(art, f'map_{key}.webp'), 'filters': [SHIFT_FILTER],
            'stats': [{'k': '引发', 'v': f'圆桌厅堂消耗为王之证 ×{cost}'}] if cost else [],
            'desc': body.split('\n')[0] if body else None, 'lore': body,
            'source': SHIFT_SOURCE, 'hand': ['source'],
            'chance': {'table': 'map', 'key': chance},
        })
    out.append({
        'id': 'map-hollow', 'name': '大空洞', 'sub': SHIFT_FILTER,
        'icon': ctx.art('wm_hollow.png', 'map_hollow.webp'), 'filters': [SHIFT_FILTER],
        'source': SHIFT_SOURCE, 'hand': ['name', 'source'],
        'chance': {'table': 'map', 'key': 'GreatHollow'},
    })
    texts = site_texts(ctx)
    for key, word, icon in SITES:
        out.append({
            'id': f'map-{key}', 'name': word, 'sub': SITE_FILTER,
            'icon': ctx.sprite(f'MENU_MenuIcon_{icon}', f'map_{key}.webp'), 'filters': [SITE_FILTER],
            'desc': texts.get(word), 'source': SITE_SOURCE, 'hand': ['source'],
        })
    return out, [SHIFT_FILTER, SITE_FILTER]
