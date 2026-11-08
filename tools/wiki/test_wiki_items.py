"""实装判断、战技分类、武器的战技段落。由 test_wiki.py 一起跑。"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cat_arts import art_choices, flat, skills  # noqa: E402
from cat_items import spells, talismans, unimplemented, weapons  # noqa: E402
from reachable import Reach, base_weapon, compute_reach  # noqa: E402


def lot(row_id, *slots):
    """slots：(类别, 物品, 权重)，没用到的格子补 0"""
    row = {'ID': row_id}
    for n in range(1, 9):
        cat, item, weight = slots[n - 1] if n <= len(slots) else (0, 0, 0)
        row.update({f'lotItemCategory0{n}': cat, f'lotItemId0{n}': item, f'lotItemBasePoint0{n}': weight})
    return row


def custom(row_id, target, arts=-1, magic=-1):
    return {'ID': row_id, 'targetWeaponId': target, 'swordArtsTableId': arts, 'magicTableId_1': magic, 'magicTableId_2': -1}


def reach_of(**tables):
    return compute_reach(lambda name: tables.get(name, []))


class Reachability(unittest.TestCase):
    def test_base_weapon_drops_variant(self):
        self.assertEqual(base_weapon(2510300), 2510000)

    def test_lot_reaches_custom_weapon(self):
        r = reach_of(ItemLotParam_enemy=[lot(1, (6, 100, 1000))], EquipParamCustomWeapon=[custom(100, 2510300)])
        self.assertEqual(r.weapons, {2510000})
        self.assertIn(2510000, r.plain)

    def test_map_lot_reaches_talisman(self):
        self.assertEqual(reach_of(ItemLotParam_map=[lot(1, (4, 8220, 10))]).talismans, {8220})

    def test_zero_weight_slot_ignored(self):
        self.assertEqual(reach_of(ItemLotParam_map=[lot(1, (4, 8220, 0))]).talismans, set())

    def test_unreferenced_custom_row_does_not_count(self):
        self.assertEqual(reach_of(EquipParamCustomWeapon=[custom(100, 2510000)]).weapons, set())

    def test_nested_tables_followed(self):
        tables = [{'ID': 5, 'itemCategory': 7, 'itemId': 6, 'chanceWeight': 1}, {'ID': 6, 'itemCategory': 4, 'itemId': 1040, 'chanceWeight': 1},
                  {'ID': 6, 'itemCategory': 4, 'itemId': 1050, 'chanceWeight': 2}]
        r = reach_of(ItemLotParam_enemy=[lot(1, (7, 5, 1))], ItemTableParam=tables)
        self.assertEqual(r.talismans, {1040, 1050})

    def test_zero_weight_table_row_ignored(self):
        tables = [{'ID': 5, 'itemCategory': 4, 'itemId': 1040, 'chanceWeight': 0}]
        self.assertEqual(reach_of(ItemLotParam_enemy=[lot(1, (7, 5, 1))], ItemTableParam=tables).talismans, set())

    def test_table_cycle_terminates(self):
        tables = [{'ID': 5, 'itemCategory': 7, 'itemId': 6, 'chanceWeight': 1}, {'ID': 6, 'itemCategory': 7, 'itemId': 5, 'chanceWeight': 1}]
        self.assertEqual(reach_of(ItemLotParam_enemy=[lot(1, (7, 5, 1))], ItemTableParam=tables).weapons, set())

    def test_shop_kinds(self):
        shop = [{'ID': 1, 'equipType': 6, 'equipId': 100}, {'ID': 2, 'equipType': 0, 'equipId': 3000100}, {'ID': 3, 'equipType': 2, 'equipId': 1040}, {'ID': 4, 'equipType': 3, 'equipId': 7}]
        r = reach_of(ShopLineupParam=shop, EquipParamCustomWeapon=[custom(100, 2510000)])
        self.assertEqual(r.weapons, {2510000, 3000000})
        self.assertEqual(r.talismans, {1040})

    def test_starting_equipment(self):
        init = [{'ID': 1, 'equip_Wep_Right_1': 33750000, 'equip_Accessory01': 3070, 'equip_Spell_01': 4000, 'equip_Spell_02': -1}]
        r = reach_of(CharaInitParam=init)
        self.assertEqual((r.weapons, r.talismans, r.spells), ({33750000}, {3070}, {4000}))
        self.assertIn(33750000, r.plain)

    def test_magic_table_gives_spells(self):
        magic = [{'ID': 9, 'magicId': 4000, 'chanceWeight': 10}, {'ID': 9, 'magicId': 4010, 'chanceWeight': 0}]
        r = reach_of(ItemLotParam_enemy=[lot(1, (6, 100, 1))], EquipParamCustomWeapon=[custom(100, 33000000, magic=9)], MagicTableParam=magic)
        self.assertEqual(r.spells, {4000})

    def test_art_table_fills_pool_not_plain(self):
        arts = [{'ID': 7, 'swordArtsId': 100, 'chanceWeight': 400}, {'ID': 7, 'swordArtsId': 101, 'chanceWeight': 0}, {'ID': 7, 'swordArtsId': -1, 'chanceWeight': 5}]
        r = reach_of(ItemLotParam_enemy=[lot(1, (6, 100, 1))], EquipParamCustomWeapon=[custom(100, 1000000, arts=7)], SwordArtsTableParam=arts)
        self.assertEqual(r.art_pool, {1000000: {100}})
        self.assertNotIn(1000000, r.plain)


class FakeCtx:
    """只实现分类函数用到的部分"""

    def __init__(self, tables, texts=None, reach=None, vanilla=None, sprites=()):
        self.tables = tables
        self.texts = texts or {}
        self.reach = reach or Reach()
        self.vanilla = vanilla or {}
        self.sprites = set(sprites)
        self.issues = []

    def rows(self, table):
        return {r['ID']: r for r in self.tables.get(table, [])}

    def vanilla_ids(self, table):
        return set(self.vanilla.get(table, []))

    def text(self, fmg, i):
        return self.texts.get((fmg, i))

    def has_sprite(self, name):
        return name in self.sprites

    def sprite(self, name, out):
        return f'img/{out}' if name in self.sprites else ''

    def chinese(self, _entry, _field, text):
        return text


def weapon(row_id, art=100, icon=1, wep_type=1, sort=1):
    row = {'ID': row_id, 'Name': '', 'iconId': icon, 'wepType': wep_type, 'sortId': sort, 'swordArtsParamId': art}
    for field in ('attackBasePhysics', 'attackBaseMagic', 'attackBaseFire', 'attackBaseThunder', 'attackBaseDark',
                  'correctStrength', 'correctAgility', 'correctMagic', 'correctFaith', 'correctLuck'):
        row[field] = 0
    return row


def art(row_id, text_id=None, icon=8300, usability=1, group=10, sort=1, cost=(-1, 16, -1, -1), name=''):
    l1, l2, r1, r2 = cost
    return {'ID': row_id, 'Name': name, 'textId': row_id if text_id is None else text_id, 'iconId': icon, 'usabilityTextId': usability, 'sortGroupId': group, 'sortId': sort,
            'useMagicPoint_L1': l1, 'useMagicPoint_L2': l2, 'useMagicPoint_R1': r1, 'useMagicPoint_R2': r2}


def art_texts(row_id, name, info='短说明', caption=None):
    return {('item/ArtsName', row_id): name, ('item/ArtsInfo', row_id): info, ('item/ArtsCaption', row_id): caption or f'战技：{name}\n\n长说明第一句。\n长说明第二句。'}


NO_SKILL = art_texts(10, '无战技')
USABLE = {('menu/CL_MenuText', 1): '能在所有的近战武器使用', ('menu/CL_MenuText', 2): '能在固有武器使用', ('menu/CL_MenuText', 3): '能在可突刺的武器使用\n（不含特大型武器）'}


class Flat(unittest.TestCase):
    def test_hard_wrap_after_punctuation_joined(self):
        self.assertEqual(flat('向前跳跃，\n旋身敲击。\n再次发动。'), '向前跳跃，旋身敲击。再次发动。')

    def test_wrap_without_punctuation_gets_comma(self):
        self.assertEqual(flat('蓄力之后向前跨出\n突破防御'), '蓄力之后向前跨出，突破防御')

    def test_bracket_line_joined(self):
        self.assertEqual(flat('能在武器使用\n（不含特大型）'), '能在武器使用（不含特大型）')

    def test_blank_line_keeps_paragraph(self):
        self.assertEqual(flat('第一段\n接着\n\n第二段'), '第一段，接着\n第二段')


class ArtChoices(unittest.TestCase):
    def ctx(self, reach, arts_rows):
        texts = {}
        for a in arts_rows:
            texts.update(art_texts(a['textId'], f'战技{a["ID"]}'))
        return FakeCtx({'SwordArtsParam': arts_rows}, texts, reach)

    def ids(self, result):
        fixed, others = result
        return fixed and fixed['ID'], [a['ID'] for a in others]

    def test_plain_weapon_has_default(self):
        c = self.ctx(Reach(plain={1000000}), [art(100)])
        self.assertEqual(self.ids(art_choices(c, weapon(1000000))), (100, []))

    def test_default_in_pool_comes_first(self):
        c = self.ctx(Reach(art_pool={1000000: {100, 101, 102}}), [art(100), art(101, sort=2), art(102, sort=3)])
        self.assertEqual(self.ids(art_choices(c, weapon(1000000))), (100, [101, 102]))

    def test_single_pool_art_wins_over_unreachable_default(self):
        c = self.ctx(Reach(art_pool={1000000: {101}}), [art(100), art(101)])
        self.assertEqual(self.ids(art_choices(c, weapon(1000000))), (101, []))

    def test_default_not_obtainable_with_big_pool(self):
        c = self.ctx(Reach(art_pool={1000000: {101, 102}}), [art(100), art(101), art(102, sort=2)])
        self.assertEqual(self.ids(art_choices(c, weapon(1000000))), (None, [101, 102]))

    def test_no_skill_never_counts(self):
        c = self.ctx(Reach(plain={1000000}), [art(1, text_id=10)])
        c.texts.update(NO_SKILL)
        self.assertEqual(self.ids(art_choices(c, weapon(1000000, art=1))), (None, []))


class WeaponEntries(unittest.TestCase):
    def ctx(self, rows, names, reach, arts_rows=(), texts=None):
        all_texts = {('item/WeaponName', i): n for i, n in names.items()}
        all_texts.update({('menu/CL_MenuText', 60010): '直剑'})
        all_texts.update(texts or {})
        return FakeCtx({'EquipParamWeapon': rows, 'SwordArtsParam': list(arts_rows)}, all_texts, reach, sprites={'MENU_ItemIcon_00001'})

    def test_unimplemented_row_not_listed(self):
        c = self.ctx([weapon(1000000), weapon(2510000)], {1000000: '匕首', 2510000: '托莉娜暗剑'}, Reach(weapons={1000000}))
        entries, _ = weapons(c)
        self.assertEqual([e['name'] for e in entries], ['匕首'])

    def test_same_name_keeps_the_obtainable_row(self):
        c = self.ctx([weapon(6850000, sort=1), weapon(68500000, sort=2)], {6850000: '野兽爪', 68500000: '野兽爪'}, Reach(weapons={68500000}))
        entries, _ = weapons(c)
        self.assertEqual([e['id'] for e in entries], ['weapon-68500000'])

    def test_sections_describe_fixed_art_and_pool(self):
        texts = {**art_texts(100, '狮子斩', '朝前方跳跃\n敲击'), **art_texts(101, '贯穿'), **art_texts(102, '突刺')}
        reach = Reach(weapons={1000000}, art_pool={1000000: {100, 101, 102}})
        c = self.ctx([weapon(1000000)], {1000000: '匕首'}, reach, [art(100), art(101, sort=2), art(102, sort=3)], texts)
        entry = weapons(c)[0][0]
        self.assertEqual(entry['sections'], [{'t': '战技 · 狮子斩', 'v': '朝前方跳跃，敲击'}, {'t': '还可能附带（2 种）', 'v': '贯穿、突刺'}])
        self.assertNotIn('战技', [s['k'] for s in entry['stats']], '战技有单独段落，数值里不重复')

    def test_pool_only_weapon_lists_pool(self):
        texts = {**art_texts(101, '贯穿'), **art_texts(102, '突刺')}
        reach = Reach(weapons={1000000}, art_pool={1000000: {101, 102}})
        c = self.ctx([weapon(1000000)], {1000000: '匕首'}, reach, [art(101), art(102, sort=2)], texts)
        self.assertEqual(weapons(c)[0][0]['sections'], [{'t': '可能附带的战技（2 种）', 'v': '贯穿、突刺'}])

    def test_pool_skips_name_of_fixed_art(self):
        texts = {**art_texts(100, '回旋斩'), **art_texts(101, '回旋斩', '另一种'), **art_texts(102, '突刺')}
        reach = Reach(weapons={1000000}, plain={1000000}, art_pool={1000000: {101, 102}})
        c = self.ctx([weapon(1000000)], {1000000: '匕首'}, reach, [art(100), art(101), art(102, sort=2)], texts)
        self.assertEqual(weapons(c)[0][0]['sections'][1], {'t': '还可能附带（1 种）', 'v': '突刺'})

    def test_pool_repeated_names_listed_once(self):
        texts = {**art_texts(101, '回旋斩'), **art_texts(102, '回旋斩', '另一种')}
        reach = Reach(weapons={1000000}, art_pool={1000000: {101, 102}})
        c = self.ctx([weapon(1000000)], {1000000: '匕首'}, reach, [art(101), art(102, sort=2)], texts)
        self.assertEqual(weapons(c)[0][0]['sections'], [{'t': '可能附带的战技（1 种）', 'v': '回旋斩'}])


class OtherItemsFiltered(unittest.TestCase):
    def test_talisman_unimplemented_dropped(self):
        row = {'ID': 8220, 'Name': '', 'sortId': 1, 'iconId': 1, 'spEffectId_1': -1, 'spEffectId_2': -1, 'spEffectId_3': -1}
        c = FakeCtx({'EquipParamAccessory': [row]}, {('item/AccessoryName', 8220): '蕾菈娜的浮雕坠饰'}, Reach())
        self.assertEqual(talismans(c)[0], [])

    def test_spell_unimplemented_dropped(self):
        rows = [{'ID': 7300, 'Name': '', 'sortId': 1, 'iconId': 1, 'ezStateBehaviorType': 1, 'mp': 0, 'requirementIntellect': 0, 'requirementFaith': 0, 'requirementLuck': 0}]
        c = FakeCtx({'Magic': rows}, {('item/MagicName', 7300): '发狂扩散'}, Reach())
        self.assertEqual(spells(c)[0], [])


class UnimplementedList(unittest.TestCase):
    def test_lists_named_rows_without_a_source(self):
        ws = [weapon(1000000), weapon(2510000), weapon(2510100), weapon(9000000, icon=0)]
        names = {('item/WeaponName', i): '名字' for i in (1000000, 2510000, 2510100, 9000000)}
        c = FakeCtx({'EquipParamWeapon': ws, 'EquipParamAccessory': [{'ID': 8220, 'iconId': 1}], 'Magic': [{'ID': 7300, 'iconId': 1}]},
                    {**names, ('item/AccessoryName', 8220): '坠饰', ('item/MagicName', 7300): '发狂扩散'}, Reach(weapons={1000000}))
        self.assertEqual(unimplemented(c), ['weapon-2510000 | 名字', 'talisman-8220 | 坠饰', 'spell-7300 | 发狂扩散'])


class SkillCategory(unittest.TestCase):
    def build(self, arts_rows, texts, reach, weapons_rows=(), names=None, sprites=('MENU_ItemIcon_08300',), vanilla=()):
        all_texts = {**USABLE, **texts, **{('item/WeaponName', i): n for i, n in (names or {}).items()}}
        c = FakeCtx({'SwordArtsParam': list(arts_rows), 'EquipParamWeapon': list(weapons_rows)}, all_texts, reach, {'SwordArtsParam': list(vanilla)}, set(sprites))
        return skills(c)

    def one(self, **kw):
        reach = Reach(weapons={1000000}, plain={1000000})
        return self.build([art(100, **kw)], art_texts(100, '狮子斩', '朝前方跳跃\n敲击'), reach, [weapon(1000000)], {1000000: '匕首'}, vanilla=[100])[0][0]

    def test_entry_fields(self):
        e = self.one()
        self.assertEqual((e['id'], e['name'], e['sub']), ('skill-100', '狮子斩', '通用战技 · 原版'))
        self.assertEqual(e['desc'], '朝前方跳跃，敲击')
        self.assertEqual(e['lore'], '长说明第一句。长说明第二句。')
        self.assertEqual(e['icon'], 'img/a_8300.webp')
        self.assertEqual(e['stats'], [{'k': '消耗专注', 'v': '16'}, {'k': '可用武器', 'v': '所有的近战武器'}, {'k': '自带武器', 'v': '匕首'}])

    def test_usability_second_line_joined(self):
        self.assertIn({'k': '可用武器', 'v': '可突刺的武器（不含特大型武器）'}, self.one(usability=3)['stats'])

    def test_fp_costs_merged(self):
        self.assertEqual(self.one(cost=(-1, 8, -1, 16))['stats'][0]['v'], '8 / 16')
        self.assertEqual(self.one(cost=(-1, 12, -1, 12))['stats'][0]['v'], '12')

    def test_unique_kind_from_usability(self):
        e = self.one(usability=2)
        self.assertEqual(e['sub'], '专用战技 · 原版')
        self.assertIn('专用', e['filters'])

    def test_unique_kind_from_caption(self):
        texts = art_texts(100, '信仰喷发', '喷发火焰', '专用战技：信仰喷发\n\n喷发大量火焰的战技。')
        e = self.build([art(100)], texts, Reach(weapons={1000000}, plain={1000000}), [weapon(1000000)], {1000000: '火杯'})[0][0]
        self.assertTrue(e['sub'].startswith('专用战技'))
        self.assertEqual(e['lore'], '喷发大量火焰的战技。')

    def test_lore_dropped_when_same_as_desc(self):
        texts = art_texts(100, '破碎居合', '进入居合，\n连续斩击。', '战技：破碎居合\n\n进入居合，\n连续斩击。')
        e = self.build([art(100)], texts, Reach(weapons={1000000}, plain={1000000}), [weapon(1000000)], {1000000: '匕首'})[0][0]
        self.assertNotIn('lore', {k for k, v in e.items() if v})

    def test_unused_art_not_listed(self):
        cats, _ = self.build([art(100), art(101)], {**art_texts(100, '狮子斩'), **art_texts(101, '没人用')}, Reach(weapons={1000000}, plain={1000000}), [weapon(1000000)], {1000000: '匕首'})
        self.assertEqual([e['name'] for e in cats], ['狮子斩'])

    def test_pool_art_listed(self):
        reach = Reach(weapons={1000000}, art_pool={1000000: {101}, 2000000: {102}})
        cats, _ = self.build([art(100), art(101), art(102, sort=2)], {**art_texts(100, '默认'), **art_texts(101, '随机一'), **art_texts(102, '另一把的')}, reach, [weapon(1000000)], {1000000: '匕首'})
        self.assertEqual([e['name'] for e in cats], ['随机一'], '不在图鉴里的武器的战技不算')

    def test_same_text_rows_merged_and_owners_combined(self):
        reach = Reach(weapons={1000000, 2000000}, plain={1000000, 2000000})
        rows = [art(110, text_id=110), art(3000, text_id=110, sort=2)]
        ws = [weapon(1000000, art=110), weapon(2000000, art=3000, sort=2)]
        cats, _ = self.build(rows, art_texts(110, '盲击'), reach, ws, {1000000: '匕首', 2000000: '长剑'})
        self.assertEqual(len(cats), 1)
        self.assertIn({'k': '自带武器', 'v': '匕首、长剑'}, cats[0]['stats'])

    def test_same_name_different_text_kept(self):
        reach = Reach(weapons={1000000, 2000000}, plain={1000000, 2000000})
        ws = [weapon(1000000, art=103), weapon(2000000, art=137, sort=2)]
        cats, _ = self.build([art(103), art(137, sort=2)], {**art_texts(103, '回旋斩', '旋转'), **art_texts(137, '回旋斩', '大回旋')}, reach, ws, {1000000: '匕首', 2000000: '长剑'})
        self.assertEqual([e['id'] for e in cats], ['skill-103', 'skill-137'])

    def test_owner_list_shortened(self):
        ws = [weapon(i * 1000000, sort=i) for i in range(1, 7)]
        reach = Reach(weapons={w['ID'] for w in ws}, plain={w['ID'] for w in ws})
        names = {w['ID']: f'武器{w["ID"] // 1000000}' for w in ws}
        e = self.build([art(100)], art_texts(100, '狮子斩'), reach, ws, names)[0][0]
        self.assertIn({'k': '自带武器', 'v': '武器1、武器2、武器3、武器4 等 6 把'}, e['stats'])

    def test_placeholder_icon_uses_owner_weapon(self):
        reach = Reach(weapons={1000000}, plain={1000000})
        e = self.build([art(100, icon=7050)], art_texts(100, '狮子斩'), reach, [weapon(1000000, icon=1)], {1000000: '匕首'}, sprites=('MENU_ItemIcon_07050', 'MENU_ItemIcon_00001'))[0][0]
        self.assertEqual(e['icon'], 'img/w_1000000.webp')

    def test_missing_icon_uses_owner_weapon(self):
        reach = Reach(weapons={1000000}, plain={1000000})
        e = self.build([art(100, icon=29800)], art_texts(100, '狮子斩'), reach, [weapon(1000000, icon=1)], {1000000: '匕首'}, sprites=('MENU_ItemIcon_00001',))[0][0]
        self.assertEqual(e['icon'], 'img/w_1000000.webp')

    def test_filters_declared(self):
        reach = Reach(weapons={1000000, 2000000}, plain={1000000, 2000000})
        ws = [weapon(1000000, art=100), weapon(2000000, art=101, sort=2)]
        cats, filters = self.build([art(100, name='[GA] 招'), art(101, usability=2, sort=2)], {**art_texts(100, '一'), **art_texts(101, '二')}, reach, ws, {1000000: '匕首', 2000000: '长剑'})
        self.assertEqual(filters, ['GA 移植', 'MMV 新增', '通用', '专用'])
        self.assertEqual(cats[0]['filters'], ['GA 移植', '通用'])


if __name__ == '__main__':
    unittest.main()
