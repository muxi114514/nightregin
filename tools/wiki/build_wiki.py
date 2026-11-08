"""图鉴导出：把 dump 产物组装成 wiki.json 和 img\\*.webp。
用法：python -I build_wiki.py <work> <out> <version> [--sprites-only]
--sprites-only 只把要用的子图名写到 <work>\\sprites.txt，给 dump_menu.ps1 决定导出哪些图集。
加分类 = 在 CATEGORIES 加一行，再写一个返回 (条目, 筛选项) 的函数。"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cat_arts import skills  # noqa: E402
from cat_items import spells, talismans, unimplemented, weapons  # noqa: E402
from cat_world import enemies, maps  # noqa: E402
from wiki_lib import HERE, WIKI_FORMAT, Ctx, apply_supplement, count_problems, icon_problems, validate  # noqa: E402

CAT_ICON_SIZE = (96, 96)
# 低于这些条数或缺图太多就算导出失败，不覆盖上一版
MIN_ENTRIES = {'weapon': 400, 'skill': 200, 'talisman': 50, 'spell': 150, 'map': 5, 'enemy': 10, 'mechanic': 5}
MAX_MISSING_ICONS = 0.05
CATEGORIES = [  # id, 名称, art 图标, 版式, 排序, 导出函数, sourceLabel
    ('weapon', '武器', 'wt_weapon.png', 'grid', 10, weapons, None),
    ('skill', '战技', 'wt_skill.png', 'grid', 15, skills, None),
    ('talisman', '护符', 'wt_talisman.png', 'grid', 20, talismans, None),
    ('spell', '法术', 'wt_spell.png', 'grid', 30, spells, None),
    ('map', '大地图', 'wm_snowup.png', 'thumbs', 40, maps, '出现'),
    ('enemy', '敌人', 'wt_enemy.png', 'grid', 50, enemies, '出现'),
    ('mechanic', '机制', 'wt_system.png', 'list', 60, lambda ctx: ([], []), '来源'),
]
# 新增条目的候选清单：表 → 条目 id 前缀、名字所在的文本表
CANDIDATES = [('EquipParamWeapon', 'weapon', 'item/WeaponName'), ('EquipParamAccessory', 'talisman', 'item/AccessoryName'), ('Magic', 'spell', 'item/MagicName')]


def build_categories(ctx):
    cats = []
    for cat_id, name, art, layout, order, export, label in CATEGORIES:
        entries, filters = export(ctx)
        cat = {'id': cat_id, 'name': name, 'icon': ctx.art(art, f'cat_{cat_id}.webp', CAT_ICON_SIZE), 'layout': layout, 'order': order}
        if label:
            cat['sourceLabel'] = label
        if filters:
            cat['filters'] = filters
        cat['entries'] = entries
        cats.append(cat)
    return cats


def resolve_supplement_icons(ctx, cats):
    """手写条目的图标写的是素材名，这里换成导出后的路径"""
    for cat in cats:
        for entry in cat['entries']:
            name = entry['id'].replace('-', '_')
            if 'iconSprite' in entry:
                entry['icon'] = ctx.sprite(entry.pop('iconSprite'), f'{name}.webp')
            elif 'iconArt' in entry:
                entry['icon'] = ctx.art(entry.pop('iconArt'), f'{name}.webp')


def strip_empty(cats):
    """空字段不写进 wiki.json"""
    for cat in cats:
        cat['entries'] = [{k: v for k, v in e.items() if v not in (None, '', [])} for e in cat['entries']]


def write_candidates(ctx, work):
    lines = []
    for table, prefix, fmg in CANDIDATES:
        previous = ctx.previous_ids(table)
        if previous is None:
            continue
        for row_id in sorted(set(ctx.rows(table)) - previous):
            name = ctx.text(fmg, row_id)
            if name:
                lines.append(f'{prefix}-{row_id} | {name}')
    with open(os.path.join(work, 'notes_candidates.txt'), 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines) + '\n')
    return len(lines)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    work, out, version = args
    sprites_only = '--sprites-only' in sys.argv
    sys.stdout.reconfigure(encoding='utf-8')
    ctx = Ctx(work, out, sprites_only)
    cats = build_categories(ctx)
    with open(os.path.join(HERE, 'supplement.json'), encoding='utf-8') as f:
        apply_supplement(cats, json.load(f))
    resolve_supplement_icons(ctx, cats)
    if sprites_only:
        with open(os.path.join(work, 'sprites.txt'), 'w', encoding='utf-8') as f:
            f.write('\n'.join(sorted(ctx.needed)) + '\n')
        print(f'sprites {len(ctx.needed)}')
        return
    strip_empty(cats)
    data = {'format': WIKI_FORMAT, 'version': version, 'categories': cats}
    with open(os.path.join(out, 'wiki.json'), 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, separators=(',', ':'))
    for cat in cats:
        print(cat['id'], len(cat['entries']))
    print('candidates', write_candidates(ctx, work))
    missing = unimplemented(ctx)
    with open(os.path.join(work, 'unimplemented.txt'), 'w', encoding='utf-8') as f:
        f.write('\n'.join(missing) + '\n')
    print('unimplemented', len(missing))
    for issue in ctx.issues:
        print('ISSUE', issue)
    problems = validate(data, out) + count_problems(cats, MIN_ENTRIES) + icon_problems(cats, MAX_MISSING_ICONS)
    for problem in problems:
        print('PROBLEM', problem)
    sys.exit(1 if problems else 0)


if __name__ == '__main__':
    main()
