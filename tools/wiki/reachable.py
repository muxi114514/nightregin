"""哪些物品玩家真的拿得到：从掉落、商店、初始装备出发沿各张表走一遍。
MMV 留了一批没接进任何来源的行（名字多带 UNUSED），只看参数表有没有行会把它们也列进图鉴。"""
from dataclasses import dataclass, field

LOT_WEAPON, LOT_ACCESSORY, LOT_CUSTOM, LOT_TABLE = 2, 4, 6, 7
SPELL = -1  # 只有初始装备会直接给法术，借一个不存在的类别号走同一个循环
SHOP_KINDS = {0: LOT_WEAPON, 2: LOT_ACCESSORY, 6: LOT_CUSTOM}
LOT_TABLES = ('ItemLotParam_enemy', 'ItemLotParam_map')
LOT_SLOTS = range(1, 9)
CHARA_PREFIXES = (('equip_Wep', LOT_WEAPON), ('equip_Accessory', LOT_ACCESSORY), ('equip_Spell', SPELL))
WEAPON_VARIANT = 10000


def base_weapon(weapon_id):
    """强化、附魔变体归到基础武器（行号末四位为 0）"""
    return weapon_id - weapon_id % WEAPON_VARIANT


@dataclass
class Reach:
    weapons: set = field(default_factory=set)  # 基础武器行号
    talismans: set = field(default_factory=set)
    spells: set = field(default_factory=set)
    plain: set = field(default_factory=set)  # 能以自带战技拿到的基础武器
    art_pool: dict = field(default_factory=dict)  # 基础武器 → 定制武器可能随机到的战技


def _by_id(rows):
    """这些表靠重复行号组成一张子表"""
    out = {}
    for row in rows:
        out.setdefault(row['ID'], []).append(row)
    return out


def _roots(table):
    for name in LOT_TABLES:
        for row in table(name):
            for n in LOT_SLOTS:
                if row[f'lotItemBasePoint0{n}'] > 0:
                    yield row[f'lotItemCategory0{n}'], row[f'lotItemId0{n}']
    for row in table('ShopLineupParam'):
        if row['equipType'] in SHOP_KINDS:
            yield SHOP_KINDS[row['equipType']], row['equipId']
    for row in table('CharaInitParam'):
        for key, value in row.items():
            for prefix, kind in CHARA_PREFIXES:
                if key.startswith(prefix):
                    yield kind, value


def _weighted(rows, key):
    return {r[key] for r in rows if r['chanceWeight'] > 0 and r[key] > 0}


def compute_reach(table):
    """table(表名) 返回该表的全部行"""
    reach = Reach()
    customs = {r['ID']: r for r in table('EquipParamCustomWeapon')}
    item_tables = _by_id(table('ItemTableParam'))
    magic_tables = _by_id(table('MagicTableParam'))
    art_tables = _by_id(table('SwordArtsTableParam'))

    def add_custom(row):
        base = base_weapon(row['targetWeaponId'])
        reach.weapons.add(base)
        for key in ('magicTableId_1', 'magicTableId_2'):
            reach.spells |= _weighted(magic_tables.get(row[key], ()), 'magicId')
        if row['swordArtsTableId'] > 0:
            reach.art_pool.setdefault(base, set()).update(_weighted(art_tables.get(row['swordArtsTableId'], ()), 'swordArtsId'))
        else:
            reach.plain.add(base)

    pending, seen = list(_roots(table)), set()
    while pending:
        kind, item = pending.pop()
        if item <= 0 or (kind, item) in seen:
            continue
        seen.add((kind, item))
        if kind == LOT_TABLE:
            pending += [(r['itemCategory'], r['itemId']) for r in item_tables.get(item, ()) if r['chanceWeight'] > 0]
        elif kind == LOT_CUSTOM:
            if item in customs:
                add_custom(customs[item])
        elif kind == LOT_WEAPON:
            reach.weapons.add(base_weapon(item))
            reach.plain.add(base_weapon(item))
        elif kind == LOT_ACCESSORY:
            reach.talismans.add(item)
        elif kind == SPELL:
            reach.spells.add(item)
    return reach
