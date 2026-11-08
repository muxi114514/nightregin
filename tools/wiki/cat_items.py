"""图鉴分类：武器、护符、法术。每个函数返回 (条目列表, 分类的筛选项)。"""
from cat_arts import art_sections
from wiki_lib import ORIGIN_GA, ORIGIN_MMV, ORIGIN_VANILLA, declared, grade, icon_sprite, is_base_weapon, origin

# wepType → CL_MenuText 类别名（60000 段）
WEP_CAT = {1: 60010, 3: 60015, 5: 60020, 7: 60025, 9: 60030, 11: 60035, 13: 60040, 14: 60043, 15: 60045, 16: 60046, 17: 60050,
           19: 60055, 21: 60060, 23: 60065, 24: 60067, 25: 60070, 28: 60077, 29: 60080, 31: 60085, 35: 60090, 37: 60100, 39: 60155,
           41: 60102, 50: 60104, 51: 60105, 53: 60110, 55: 60115, 56: 60117, 57: 60120, 61: 60130, 65: 60140, 67: 60145, 69: 60150,
           87: 60175, 90: 60180, 91: 60181, 92: 60182, 93: 60183, 94: 60184, 95: 60185, 96: 60186, 97: 60187}
ATK = [('attackBasePhysics', '物理'), ('attackBaseMagic', '魔力'), ('attackBaseFire', '火'), ('attackBaseThunder', '雷'), ('attackBaseDark', '圣')]
COR = [('correctStrength', '力'), ('correctAgility', '技'), ('correctMagic', '智'), ('correctFaith', '信'), ('correctLuck', '感')]
REQ = [('requirementIntellect', '智'), ('requirementFaith', '信'), ('requirementLuck', '感')]
MAGIC_KIND = {0: '魔法', 1: '祷告'}
ORIGIN_FILTERS = [ORIGIN_GA, ORIGIN_MMV]


def split_caption(caption):
    """第一行作正文，其余作 lore"""
    if not caption:
        return None, None
    first, _, rest = caption.partition('\n')
    return first.strip(), rest.strip() or None


def first_by_name(rows, name_of):
    """同名的行只留第一条"""
    seen, out = set(), []
    for row in rows:
        name = name_of(row['ID'])
        if name and name not in seen and row.get('iconId', 0) > 0:
            seen.add(name)
            out.append((row, name))
    return out


def unimplemented(ctx):
    """有名字和图标编号、却没有任何获取途径的行，导出时写一份备查"""
    out = []
    for table, prefix, fmg, found in (('EquipParamWeapon', 'weapon', 'item/WeaponName', ctx.reach.weapons),
                                      ('EquipParamAccessory', 'talisman', 'item/AccessoryName', ctx.reach.talismans),
                                      ('Magic', 'spell', 'item/MagicName', ctx.reach.spells)):
        for i, row in sorted(ctx.rows(table).items()):
            name = ctx.text(fmg, i)
            if name and i not in found and row.get('iconId', 0) > 0 and (prefix != 'weapon' or is_base_weapon(i)):
                out.append(f'{prefix}-{i} | {name}')
    return out


def weapons(ctx):
    table = 'EquipParamWeapon'
    rows, vanilla = ctx.rows(table), ctx.vanilla_ids(table)
    order = {t: i for i, t in enumerate(WEP_CAT)}
    # 先筛实装再按名字去重，同名时才会留下拿得到的那一行
    base = sorted((r for i, r in rows.items() if is_base_weapon(i) and i in ctx.reach.weapons),
                  key=lambda r: (order.get(r['wepType'], len(order)), r['sortId'], r['ID']))
    out = []
    for r, name in first_by_name(base, lambda i: ctx.text('item/WeaponName', i)):
        i = r['ID']
        entry_id = f'weapon-{i}'
        cat = ctx.text('menu/CL_MenuText', WEP_CAT.get(r['wepType'], -1))
        if not cat:
            ctx.issues.append(f'{entry_id}: wepType {r["wepType"]} 没有类别名')
        tag = origin(r, vanilla)
        attack = [f'{label} {r[field]}' for field, label in ATK if r[field] > 0]
        correct = [f'{label} {grade(r[field])}' for field, label in COR if grade(r[field])]
        stats = [('类别', cat), ('基础攻击力', ' / '.join(attack)), ('属性补正', ' / '.join(correct))]
        desc, lore = split_caption(ctx.chinese(entry_id, 'caption', ctx.text('item/WeaponCaption', i)))
        out.append({
            'id': entry_id, 'name': name, 'sub': ' · '.join(x for x in (cat, tag) if x),
            'icon': ctx.sprite(icon_sprite(r['iconId']), f'w_{i}.webp'),
            'filters': [x for x in ([tag] if tag != ORIGIN_VANILLA else []) + [cat] if x],
            'stats': [{'k': k, 'v': v} for k, v in stats if v],
            'desc': desc, 'lore': lore, 'sections': art_sections(ctx, r),
        })
    return out, declared(out, ORIGIN_FILTERS)


def talismans(ctx):
    table = 'EquipParamAccessory'
    rows, effects, vanilla = ctx.rows(table), ctx.rows('AttachEffectParam'), ctx.vanilla_ids(table)
    out = []
    ordered = sorted((r for i, r in rows.items() if i in ctx.reach.talismans), key=lambda r: (r['sortId'], r['ID']))
    for r, name in first_by_name(ordered, lambda i: ctx.text('item/AccessoryName', i)):
        i = r['ID']
        entry_id = f'talisman-{i}'
        tag = origin(r, vanilla)
        stats = []
        for n, slot in enumerate(('spEffectId_1', 'spEffectId_2', 'spEffectId_3'), 1):
            effect = effects.get(r[slot])
            text = effect and ctx.text('item/AttachEffectName', effect['attachTextId'])
            if text:
                stats.append({'k': f'效果 {n}', 'v': text})
        out.append({
            'id': entry_id, 'name': name, 'sub': f'护符 · {tag}',
            'icon': ctx.sprite(icon_sprite(r['iconId']), f't_{i}.webp'),
            'filters': [tag], 'stats': stats,
            'desc': ctx.chinese(entry_id, 'info', ctx.text('item/AccessoryInfo', i)),
            'lore': ctx.chinese(entry_id, 'caption', ctx.text('item/AccessoryCaption', i)),
        })
    return out, declared(out, [ORIGIN_GA, ORIGIN_MMV, ORIGIN_VANILLA])


def spells(ctx):
    table = 'Magic'
    rows, vanilla = ctx.rows(table), ctx.vanilla_ids(table)
    out = []
    ordered = sorted((r for i, r in rows.items() if i in ctx.reach.spells), key=lambda r: (r['ezStateBehaviorType'], r['sortId'], r['ID']))
    for r, name in first_by_name(ordered, lambda i: ctx.text('item/MagicName', i)):
        i = r['ID']
        entry_id = f'spell-{i}'
        kind = MAGIC_KIND.get(r['ezStateBehaviorType'])
        if not kind:
            ctx.issues.append(f'{entry_id}: 类型 {r["ezStateBehaviorType"]} 不认识')
        need = [f'{label} {r[field]}' for field, label in REQ if r[field] > 0]
        stats = [('消耗专注', str(r['mp']) if r['mp'] > 0 else None), ('需求', ' / '.join(need))]
        caption = ctx.chinese(entry_id, 'caption', ctx.text('item/MagicCaption', i))
        info = ctx.chinese(entry_id, 'info', ctx.text('item/MagicInfo', i))
        first, rest = split_caption(caption)
        out.append({
            'id': entry_id, 'name': name, 'sub': ' · '.join(x for x in (kind, origin(r, vanilla)) if x),
            'icon': ctx.sprite(icon_sprite(r['iconId']), f's_{i}.webp'),
            'filters': [kind] if kind else [],
            'stats': [{'k': k, 'v': v} for k, v in stats if v],
            'desc': info or first, 'lore': caption if info else rest,
        })
    return out, declared(out, list(MAGIC_KIND.values()))
