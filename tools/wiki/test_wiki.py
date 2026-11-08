"""图鉴导出的纯逻辑测试：内联数据和临时目录，不碰游戏文件。用法：python -I tools\\wiki\\test_wiki.py"""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_wiki_items import *  # noqa: E402,F401,F403  一条命令跑全部
from wiki_lib import (  # noqa: E402
    apply_supplement, count_problems, grade, has_cjk, icon_problems, install_dir, is_base_weapon, origin, validate,
)


def entry(**kw):
    return {'id': 'weapon-1', 'name': '阎魔刀', **kw}


def category(entries, **kw):
    return {'id': 'weapon', 'name': '武器', 'layout': 'grid', 'order': 10, 'entries': entries, **kw}


def data(cats):
    return {'format': 1, 'version': '1.9.5', 'categories': cats}


class Basics(unittest.TestCase):
    def test_grade_thresholds(self):
        cases = {0: None, 24: 'E', 25: 'D', 60: 'C', 90: 'B', 140: 'A', 175: 'S'}
        for v, g in cases.items():
            self.assertEqual(grade(v), g, v)

    def test_is_base_weapon(self):
        self.assertTrue(is_base_weapon(1100000))
        self.assertFalse(is_base_weapon(1100500))

    def test_has_cjk(self):
        self.assertTrue(has_cjk('阎魔刀'))
        self.assertFalse(has_cjk('A black blade'))

    def test_origin(self):
        self.assertEqual(origin({'ID': 5, 'Name': '[GA] Weapon'}, {1}), 'GA 移植')
        self.assertEqual(origin({'ID': 1, 'Name': 'x'}, {1}), '原版')
        self.assertEqual(origin({'ID': 5, 'Name': 'x'}, {1}), 'MMV 新增')


class Supplement(unittest.TestCase):
    def test_patch_marks_hand(self):
        e = entry(source='旧')
        apply_supplement([category([e])], {'patch': {'weapon-1': {'source': '首领掉落球'}}})
        self.assertEqual(e['source'], '首领掉落球')
        self.assertIn('source', e['hand'])

    def test_patch_note_not_hand(self):
        e = entry()
        apply_supplement([category([e])], {'patch': {'weapon-1': {'note': '备注'}}})
        self.assertEqual(e['note'], '备注')
        self.assertNotIn('note', e.get('hand', []))

    def test_patch_stats_marks_each_hand(self):
        e = entry()
        apply_supplement([category([e])], {'patch': {'weapon-1': {'stats': [{'k': '效果', 'v': 'x'}]}}})
        self.assertTrue(e['stats'][0]['h'])

    def test_patch_unknown_id_raises(self):
        with self.assertRaises(ValueError):
            apply_supplement([category([entry()])], {'patch': {'weapon-2': {'source': 'x'}}})

    def test_entries_appended_all_hand(self):
        c = category([])
        apply_supplement([c], {'entries': {'weapon': [entry(sub='副标题', desc='正文', source='来源')]}})
        self.assertEqual(len(c['entries']), 1)
        self.assertEqual(set(c['entries'][0]['hand']), {'name', 'sub', 'desc', 'source'})

    def test_entries_stats_marked_hand(self):
        c = category([])
        apply_supplement([c], {'entries': {'weapon': [entry(stats=[{'k': '规则', 'v': 'x'}])]}})
        self.assertTrue(c['entries'][0]['stats'][0]['h'])

    def test_entries_unknown_category_raises(self):
        with self.assertRaises(ValueError):
            apply_supplement([category([])], {'entries': {'nope': [entry()]}})


class Validate(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.out = tmp.name
        os.makedirs(os.path.join(self.out, 'img'))
        open(os.path.join(self.out, 'img', 'a.webp'), 'wb').close()

    def check(self, entries, **kw):
        return validate(data([category(entries, **kw)]), self.out)

    def test_validate_ok(self):
        self.assertEqual(self.check([entry(icon='img/a.webp', filters=['刀'])], filters=['刀', '大剑']), [])

    def test_validate_duplicate_id(self):
        self.assertTrue(self.check([entry(), entry()]))

    def test_validate_missing_image(self):
        self.assertTrue(self.check([entry(icon='img/none.webp')]))

    def test_validate_placeholder_text(self):
        self.assertTrue(self.check([entry(source='（占位）获取方式待补')]))

    def test_validate_filter_not_declared(self):
        self.assertTrue(self.check([entry(filters=['刀'])], filters=['大剑', '斧']))

    def test_validate_unknown_layout(self):
        self.assertTrue(self.check([entry()], layout='hex'))

    def test_validate_dev_text(self):
        self.assertTrue(self.check([entry(stats=[{'k': '概率', 'v': 'boss_loot.smithing_stone_percent'}])]))

    def test_validate_note_may_have_dev_text(self):
        self.assertEqual(self.check([entry(note='boss_loot.smithing_stone_percent 权重')]), [])

    def test_validate_empty_category(self):
        self.assertTrue(self.check([]))

    def test_validate_wrong_format(self):
        self.assertTrue(validate({'format': 2, 'categories': []}, self.out))

    def test_validate_null_text(self):
        self.assertTrue(self.check([entry(desc='%null%')]))

    def test_validate_section_text(self):
        self.assertTrue(self.check([entry(sections=[{'t': '战技', 'v': '%null%'}])]))


class ExportGates(unittest.TestCase):
    def test_count_below_minimum(self):
        self.assertTrue(count_problems([category([entry()])], {'weapon': 2}))

    def test_count_enough(self):
        self.assertEqual(count_problems([category([entry()])], {'weapon': 1}), [])

    def test_icons_mostly_missing(self):
        cats = [category([entry(id=f'w-{i}') for i in range(10)])]
        self.assertTrue(icon_problems(cats, 0.05))

    def test_icons_few_missing(self):
        entries = [entry(id=f'w-{i}', icon='img/a.webp') for i in range(99)] + [entry(id='w-x')]
        self.assertEqual(icon_problems([category(entries)], 0.05), [])


class Install(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = tmp.name
        self.src = os.path.join(self.root, 'src')
        self.target = os.path.join(self.root, 'data')
        self.write(self.src, 'new')

    def write(self, folder, text):
        os.makedirs(folder, exist_ok=True)
        with open(os.path.join(folder, 'wiki.json'), 'w', encoding='utf-8') as f:
            f.write(text)

    def read(self, folder):
        with open(os.path.join(folder, 'wiki.json'), encoding='utf-8') as f:
            return f.read()

    def install(self, rename=os.rename):
        install_dir(self.src, self.target, rename=rename, pause=lambda _: None)

    def test_install_replaces(self):
        self.write(self.target, 'old')
        self.install()
        self.assertEqual(self.read(self.target), 'new')
        self.assertEqual(sorted(os.listdir(self.root)), ['data', 'src'])

    def test_install_first_time(self):
        self.install()
        self.assertEqual(self.read(self.target), 'new')

    def test_install_restores_on_failure(self):
        self.write(self.target, 'old')

        def flaky(src, dst):
            if src.endswith('.new'):
                raise PermissionError('杀软占用')
            os.rename(src, dst)

        with self.assertRaises(OSError):
            self.install(flaky)
        self.assertEqual(self.read(self.target), 'old')

    def test_install_retries_transient(self):
        self.write(self.target, 'old')
        calls = []

        def flaky(src, dst):
            calls.append(src)
            if src.endswith('.new') and calls.count(src) < 3:
                raise PermissionError('杀软占用')
            os.rename(src, dst)

        self.install(flaky)
        self.assertEqual(self.read(self.target), 'new')

    def test_install_recovers_leftover_old(self):
        self.write(self.target + '.old', 'old')
        with self.assertRaises(OSError):
            self.install(lambda s, d: (_ for _ in ()).throw(PermissionError('x')) if s.endswith('.new') else os.rename(s, d))
        self.assertEqual(self.read(self.target), 'old', '上次断在中间留下的 old 不能丢')


if __name__ == '__main__':
    unittest.main()
