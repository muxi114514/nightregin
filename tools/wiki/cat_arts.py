"""战技：单独一个分类，武器说明里也带上。
黑夜君临没有战灰物品，武器的战技要么是自带的，要么来自定制武器的随机池。"""
import re

from wiki_lib import ORIGIN_GA, ORIGIN_MMV, ORIGIN_VANILLA, declared, icon_sprite, origin

NO_SKILL_TEXT = 10  # 「无战技」的文本号
PLACEHOLDER_ICON = 7050  # 游戏给没有专属图标的战技用的人偶图
MAX_OWNERS = 4
KIND_COMMON, KIND_UNIQUE = '通用', '专用'
UNIQUE_MARKS = ('专用', '专属')
FP_FIELDS = ('useMagicPoint_L1', 'useMagicPoint_L2', 'useMagicPoint_R1', 'useMagicPoint_R2')


def _flat_paragraph(text):
    text = re.sub(r'([，。、；：！？）])\n', r'\1', text)
    text = re.sub(r'\n(?=（)', '', text)
    return text.replace('\n', '，')


def flat(text):
    """游戏文本按行宽硬换行，这里拼回整段；原本只靠换行断句的地方补逗号"""
    return '\n'.join(_flat_paragraph(p) for p in text.split('\n\n'))


def art_text(ctx, art):
    """(名字, 短说明, 长说明)；无战技、没有名字的行名字为 None"""
    text_id = art['textId']
    name = None if text_id == NO_SKILL_TEXT else ctx.text('item/ArtsName', text_id)
    caption = ctx.text('item/ArtsCaption', text_id) or ''
    head, gap, body = caption.partition('\n\n')
    full = flat(body if gap and '战技' in head else caption)
    short = flat(ctx.text('item/ArtsInfo', text_id) or '')
    return name, short or full or None, full or None


def _named(ctx, art):
    return art is not None and art_text(ctx, art)[0] is not None


def _order(art):
    return art['sortGroupId'], art['sortId'], art['ID']


def art_choices(ctx, weapon):
    """(固定战技的行或 None, 其余可能随机到的战技行)"""
    arts = ctx.rows('SwordArtsParam')
    reach = ctx.reach
    pool = {i for i in reach.art_pool.get(weapon['ID'], ()) if _named(ctx, arts.get(i))}
    default = weapon['swordArtsParamId']
    fixed = None
    if _named(ctx, arts.get(default)) and (weapon['ID'] in reach.plain or default in pool):
        fixed = default
    elif len(pool) == 1:
        fixed = next(iter(pool))
    return arts[fixed] if fixed is not None else None, sorted((arts[i] for i in pool if i != fixed), key=_order)


def art_sections(ctx, weapon):
    """武器说明里的战技段落：固定战技的说明 + 随机池里的其余名字"""
    fixed, others = art_choices(ctx, weapon)
    sections = []
    fixed_name = None
    if fixed:
        fixed_name, short, _ = art_text(ctx, fixed)
        sections.append({'t': f'战技 · {fixed_name}', 'v': short or fixed_name})
    names = [n for n in dict.fromkeys(art_text(ctx, a)[0] for a in others) if n != fixed_name]
    if names:
        title = f'还可能附带（{len(names)} 种）' if fixed else f'可能附带的战技（{len(names)} 种）'
        sections.append({'t': title, 'v': '、'.join(names)})
    return sections


def _carriers(ctx):
    """(战技 → 自带它的武器行, 拿得到的武器上出现过的全部战技)"""
    rows = ctx.rows('EquipParamWeapon')
    weapons = [rows[i] for i in ctx.reach.weapons if i in rows and ctx.text('item/WeaponName', i)]
    owners, used = {}, set()
    for row in sorted(weapons, key=lambda r: (r['sortId'], r['ID'])):
        fixed, others = art_choices(ctx, row)
        if fixed:
            owners.setdefault(fixed['ID'], []).append(row)
            used.add(fixed['ID'])
        used.update(a['ID'] for a in others)
    return owners, used


def _kind(ctx, art):
    head = (ctx.text('item/ArtsCaption', art['textId']) or '').partition('\n')[0]
    usable = ctx.text('menu/CL_MenuText', art['usabilityTextId']) or ''
    return KIND_UNIQUE if '固有' in usable or any(m in head for m in UNIQUE_MARKS) else KIND_COMMON


def _fp(art):
    return ' / '.join(str(c) for c in sorted({art[f] for f in FP_FIELDS if art[f] >= 0})) or None


def _usable_on(ctx, art):
    text = ctx.text('menu/CL_MenuText', art['usabilityTextId'])
    return flat(text).removeprefix('能在').replace('使用', '') if text else None


def _owner_names(ctx, carriers):
    names = list(dict.fromkeys(ctx.text('item/WeaponName', w['ID']) for w in carriers))
    if len(names) <= MAX_OWNERS:
        return '、'.join(names)
    return f'{"、".join(names[:MAX_OWNERS])} 等 {len(names)} 把'


def _icon(ctx, art, carriers):
    """没有专属图标的战技借自带它的武器的图标，不然几十格都是同一个人偶"""
    sprite = icon_sprite(art['iconId'])
    if (art['iconId'] != PLACEHOLDER_ICON and ctx.has_sprite(sprite)) or not carriers:
        return ctx.sprite(sprite, f'a_{art["iconId"]}.webp')
    weapon = carriers[0]
    return ctx.sprite(icon_sprite(weapon['iconId']), f'w_{weapon["ID"]}.webp')


def _entry(ctx, arts, owners, vanilla):
    art = arts[0]
    name, short, full = art_text(ctx, art)
    entry_id = f'skill-{art["ID"]}'
    carriers = [w for a in arts for w in owners.get(a['ID'], [])]
    kind, tag = _kind(ctx, art), origin(art, vanilla)
    stats = [('消耗专注', _fp(art)), ('可用武器', _usable_on(ctx, art)), ('自带武器', _owner_names(ctx, carriers))]
    return {
        'id': entry_id, 'name': name, 'sub': f'{kind}战技 · {tag}',
        'icon': _icon(ctx, art, carriers),
        'filters': ([tag] if tag != ORIGIN_VANILLA else []) + [kind],
        'stats': [{'k': k, 'v': v} for k, v in stats if v],
        'desc': ctx.chinese(entry_id, 'info', short),
        'lore': None if full == short else ctx.chinese(entry_id, 'caption', full),
    }


def skills(ctx):
    table = 'SwordArtsParam'
    rows, vanilla = ctx.rows(table), ctx.vanilla_ids(table)
    owners, used = _carriers(ctx)
    # 不同行用同一段文字（换武器的同招）算一条；同名但说明不同的分开列
    groups = {}
    for art in sorted((rows[i] for i in used), key=_order):
        name, short, _ = art_text(ctx, art)
        groups.setdefault((name, short), []).append(art)
    out = [_entry(ctx, arts, owners, vanilla) for arts in groups.values()]
    return out, declared(out, [ORIGIN_GA, ORIGIN_MMV, KIND_COMMON, KIND_UNIQUE])
