"""图鉴导出的公共部分：读 dump 产物、存图、合并手写补充、校验 wiki.json。"""
import json
import os
import re
import shutil
import sys
import time
from functools import cached_property

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from menu_sprites import Sprites  # noqa: E402
from PIL import Image  # noqa: E402
from reachable import compute_reach  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ART_DIR = os.path.join(HERE, 'art')
WIKI_FORMAT = 1
LAYOUTS = ('grid', 'thumbs', 'list')
HAND_FIELDS = ('name', 'sub', 'desc', 'lore', 'source')
ICON_SIZE = (160, 160)
PORTRAIT_SIZE = (320, 320)
GA_PREFIX = '[GA]'
ORIGIN_GA = 'GA 移植'
ORIGIN_VANILLA = '原版'
ORIGIN_MMV = 'MMV 新增'
# 不该出现在界面上的文字
BAD_WORDS = ('（占位）', '待补', '权重', '脚本', '%null%')
DEV_TEXT = re.compile(r'[a-z]+_[a-z_]+')
RENAME_RETRIES = 5
RENAME_PAUSE = 0.5


def grade(v):
    """属性补正的字母档；没有补正时为 None"""
    if v <= 0:
        return None
    for limit, letter in ((175, 'S'), (140, 'A'), (90, 'B'), (60, 'C'), (25, 'D')):
        if v >= limit:
            return letter
    return 'E'


def is_base_weapon(row_id):
    """基础武器：行号末四位为 0；火焰、雷电等附魔变体不单列"""
    return row_id % 10000 == 0


def has_cjk(text):
    return any('一' <= ch <= '鿿' for ch in text)


def origin(row, vanilla_ids):
    if str(row.get('Name', '')).startswith(GA_PREFIX):
        return ORIGIN_GA
    return ORIGIN_VANILLA if row['ID'] in vanilla_ids else ORIGIN_MMV


def icon_sprite(icon_id):
    return f'MENU_ItemIcon_{icon_id:05d}'


def declared(entries, leading):
    """筛选项：固定的几项在前（有条目才列），其余按第一次出现的顺序"""
    used = {f for e in entries for f in e.get('filters', [])}
    head = [f for f in leading if f in used]
    tail = []
    for e in entries:
        for f in e.get('filters', []):
            if f not in head and f not in tail:
                tail.append(f)
    return head + tail


def _mark_hand(entry, fields):
    hand = entry.get('hand', [])
    hand += [f for f in fields if f in HAND_FIELDS and f not in hand]
    if hand:
        entry['hand'] = hand


def apply_supplement(cats, sup):
    """patch 覆盖字段并标手写；entries 整条追加，可见字段都标手写。对不上号直接报错，免得手写内容悄悄失效"""
    index = {e['id']: e for c in cats for e in c['entries']}
    for entry_id, patch in sup.get('patch', {}).items():
        entry = index.get(entry_id)
        if entry is None:
            raise ValueError(f'supplement 里的条目不存在：{entry_id}')
        for key, value in patch.items():
            entry[key] = [{**s, 'h': True} for s in value] if key == 'stats' else value
        _mark_hand(entry, patch)
    by_id = {c['id']: c for c in cats}
    for cat_id, entries in sup.get('entries', {}).items():
        if cat_id not in by_id:
            raise ValueError(f'supplement 里的分类不存在：{cat_id}')
        for source in entries:
            entry = dict(source)
            if 'stats' in entry:
                entry['stats'] = [{**s, 'h': True} for s in entry['stats']]
            _mark_hand(entry, entry)
            by_id[cat_id]['entries'].append(entry)


def _visible_texts(entry):
    for field in HAND_FIELDS:
        if entry.get(field):
            yield entry[field]
    for stat in entry.get('stats', []):
        yield stat['k']
        yield stat['v']
    for section in entry.get('sections', []):
        yield section['t']
        yield section['v']


def _check_text(entry_id, text, problems):
    for word in BAD_WORDS:
        if word in text:
            problems.append(f'{entry_id}: 可见文字含「{word}」：{text[:40]}')
    if DEV_TEXT.search(text):
        problems.append(f'{entry_id}: 可见文字含开发信息：{text[:40]}')


def validate(data, out_dir):
    """返回问题清单，空表示合格"""
    problems = []
    if data.get('format') != WIKI_FORMAT:
        problems.append(f'format 不是 {WIKI_FORMAT}')
    seen = set()

    def check_image(owner, rel):
        if rel and not os.path.isfile(os.path.join(out_dir, rel)):
            problems.append(f'{owner}: 图片不存在：{rel}')

    for cat in data.get('categories', []):
        if cat.get('layout') not in LAYOUTS:
            problems.append(f'{cat["id"]}: 版式不认识：{cat.get("layout")}')
        check_image(cat['id'], cat.get('icon'))
        if not cat.get('entries'):
            problems.append(f'{cat["id"]}: 没有条目')
        declared = set(cat.get('filters', []))
        for entry in cat.get('entries', []):
            entry_id = entry['id']
            if entry_id in seen:
                problems.append(f'条目 id 重复：{entry_id}')
            seen.add(entry_id)
            for field in ('icon', 'image', 'everIcon'):
                check_image(entry_id, entry.get(field))
            for name in entry.get('filters', []):
                if name not in declared:
                    problems.append(f'{entry_id}: 筛选项没有在分类里声明：{name}')
            for text in _visible_texts(entry):
                _check_text(entry_id, text, problems)
    return problems


def count_problems(cats, minimums):
    """条目数低于下限：多半是文本层或参数没读全，别拿空壳覆盖上一版"""
    return [f'{c["id"]}: 只有 {len(c["entries"])} 条，至少要 {minimums[c["id"]]} 条'
            for c in cats if len(c['entries']) < minimums.get(c['id'], 0)]


def icon_problems(cats, limit):
    """一个分类里缺图的比例超过 limit：多半是图集没导出来"""
    problems = []
    for cat in cats:
        total = len(cat['entries'])
        missing = sum(1 for e in cat['entries'] if not e.get('icon'))
        if total and missing / total > limit:
            problems.append(f'{cat["id"]}: {missing} / {total} 条没有图标')
    return problems


def _rename_retry(rename, pause, src, dst):
    """杀软可能正在扫刚写的文件，临时性失败退避重试"""
    for attempt in range(RENAME_RETRIES):
        try:
            rename(src, dst)
            return
        except PermissionError:
            if attempt == RENAME_RETRIES - 1:
                raise
            pause(RENAME_PAUSE)


def install_dir(src, target, rename=os.rename, pause=time.sleep):
    """把 src 整体换到 target；中途失败时 target 还是旧内容，上次断在半路留下的 .old 先还原"""
    new, old = target + '.new', target + '.old'
    if os.path.isdir(old):
        if os.path.isdir(target):
            shutil.rmtree(old)
        else:
            _rename_retry(rename, pause, old, target)
    if os.path.isdir(new):
        shutil.rmtree(new)
    shutil.copytree(src, new)
    had_old = os.path.isdir(target)
    if had_old:
        _rename_retry(rename, pause, target, old)
    try:
        _rename_retry(rename, pause, new, target)
    except OSError:
        if had_old:
            _rename_retry(rename, pause, old, target)
        raise
    if had_old:
        shutil.rmtree(old, ignore_errors=True)


class Ctx:
    """持有 dump 产物和输出目录；sprites_only 时只收集要用的子图名，不读贴图"""

    def __init__(self, work, out, sprites_only=False):
        with open(os.path.join(work, 'text.json'), encoding='utf-8') as f:
            self._text = json.load(f)['zhocn']
        with open(os.path.join(work, 'params.json'), encoding='utf-8') as f:
            self.params = json.load(f)
        self._sprites = Sprites(os.path.join(work, 'menu'))
        self.img_dir = os.path.join(out, 'img')
        self.sprites_only = sprites_only
        self.needed = set()
        self.issues = []
        os.makedirs(self.img_dir, exist_ok=True)

    def text(self, fmg, i):
        """先本体后 _dlc01；空串和 %null% 当没有"""
        for key in (fmg, fmg + '_dlc01'):
            value = self._text.get(key, {}).get(str(i))
            if value and value.strip() and value.strip() != '%null%':
                return value.strip()
        return None

    def rows(self, table):
        return {r['ID']: r for r in self.params['pkg'][table]}

    def table(self, name):
        """整张表按行返回：ItemTableParam 这类靠重复行号组成子表，不能按行号建字典"""
        return self.params['pkg'][name]

    @cached_property
    def reach(self):
        return compute_reach(self.table)

    def has_sprite(self, name):
        return self._sprites.where(name) is not None

    def vanilla_ids(self, table):
        return set(self.params['ids']['vanilla'][table])

    def previous_ids(self, table):
        previous = self.params['ids'].get('previous')
        return set(previous[table]) if previous else None

    def _save(self, img, out, size, quality):
        img = img.copy()
        img.thumbnail(size, Image.LANCZOS)
        img.save(os.path.join(self.img_dir, out), 'WEBP', quality=quality, method=6)
        return 'img/' + out

    def sprite(self, name, out, size=ICON_SIZE, quality=86):
        """返回 img/… 的相对路径；缺图记 issues，返回 ""。"""
        if self.sprites_only:
            self.needed.add(name)
            return 'img/' + out
        img = self._sprites.image(name)
        if img is None:
            self.issues.append(f'缺图：{name}')
            return ''
        return self._save(img, out, size, quality)

    def art(self, file, out, size=ICON_SIZE):
        """tools\\wiki\\art 里的贴图"""
        if self.sprites_only:
            return 'img/' + out
        with Image.open(os.path.join(ART_DIR, file)) as img:
            return self._save(img.convert('RGBA'), out, size, 86)

    def chinese(self, entry_id, field, text):
        """没有中文的说明丢掉（有的条目中文包里还是英文），记进 issues"""
        if text and not has_cjk(text):
            self.issues.append(f'{entry_id}: {field} 没有中文，已丢弃')
            return None
        return text
