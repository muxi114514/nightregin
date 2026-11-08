"""菜单图集子图的查找与裁切（只读游戏文件，依赖 dump_menu.ps1 导出的布局与 DDS）。
用法：
  python -I menu_sprites.py plan <menu目录> 子图名...        列出每个子图所在的包与图集（给 dump_menu.ps1 -Atlases 用）
  python -I menu_sprites.py sheet <menu目录> <输出.png> 正则    把名字匹配的子图拼成总览图
  其他脚本 import 后用 Sprites(menu目录).image(子图名)
包的查找顺序同 dump_menu.ps1：pkg → mmv → mmv_dlc → van → van_dlc，取第一个有该子图的包。
"""
import io
import os
import re
import struct
import sys

from PIL import Image, ImageDraw

TAGS = ['pkg', 'mmv', 'mmv_dlc', 'van', 'van_dlc']
BLOCK8 = {71, 72, 80, 81}
ATTR = re.compile(r'(\w+)="([^"]*)"')
SUB = re.compile(r'<SubTexture [^>]*>')
ATLAS = re.compile(r'imagePath="([^"]+)"')


def dds_info(raw):
    h, w = struct.unpack_from('<II', raw, 12)
    if raw[84:88] == b'DX10':
        fmt = struct.unpack_from('<I', raw, 128)[0]
        return w, h, 148, (8 if fmt in BLOCK8 else 16)
    return w, h, 128, (8 if raw[84:88] == b'DXT1' else 16)


def crop(raw, x, y, w, h):
    """照 ga_art_png.py：按 4x4 块对齐向外取整后裁出、解码，再精确裁到目标矩形"""
    width, height, head, block = dds_info(raw)
    ax, ay = x // 4 * 4, y // 4 * 4
    aw = min((x + w + 3) // 4 * 4, width) - ax
    ah = min((y + h + 3) // 4 * 4, height) - ay
    row = (width + 3) // 4
    out = bytearray()
    for by in range(ay // 4, (ay + ah) // 4):
        at = head + (by * row + ax // 4) * block
        out += raw[at:at + (aw // 4) * block]
    hdr = bytearray(raw[:head])
    struct.pack_into('<II', hdr, 12, ah, aw)
    struct.pack_into('<I', hdr, 20, len(out) // (ah // 4))
    struct.pack_into('<I', hdr, 28, 1)
    img = Image.open(io.BytesIO(bytes(hdr) + bytes(out))).convert('RGBA')
    return img.crop((x - ax, y - ay, x - ax + w, y - ay + h))


class Sprites:
    def __init__(self, menu_dir):
        self.menu_dir = menu_dir
        self.index = {}  # 子图名（不带 .png）→ (包, 图集, x, y, w, h)
        self._raw = {}
        for tag in TAGS:
            lay_dir = os.path.join(menu_dir, tag, 'layouts')
            if not os.path.isdir(lay_dir):
                continue
            for fn in sorted(os.listdir(lay_dir)):
                text = open(os.path.join(lay_dir, fn), 'rb').read().decode('utf-8', 'replace')
                m = ATLAS.search(text)
                atlas = os.path.splitext(os.path.basename(m.group(1).replace('\\', '/')))[0] if m else os.path.splitext(fn)[0]
                for s in SUB.finditer(text):
                    a = dict(ATTR.findall(s.group(0)))
                    name = os.path.splitext(a['name'])[0]
                    if name not in self.index:
                        self.index[name] = (tag, atlas, int(a['x']), int(a['y']), int(a['width']), int(a['height']))

    def where(self, name):
        return self.index.get(name)

    def image(self, name):
        hit = self.index.get(name)
        if not hit:
            return None
        tag, atlas, x, y, w, h = hit
        path = os.path.join(self.menu_dir, tag, 'dds', atlas + '.dds')
        if not os.path.exists(path):
            return None
        if path not in self._raw:
            self._raw[path] = open(path, 'rb').read()
        return crop(self._raw[path], x, y, w, h)


def main():
    mode, menu_dir = sys.argv[1], sys.argv[2]
    sp = Sprites(menu_dir)
    if mode == 'plan':
        need = {}
        for name in sys.argv[3:]:
            hit = sp.where(name)
            print(name, hit)
            if hit:
                need.setdefault(hit[0], set()).add(hit[1])
        for tag, atlases in need.items():
            print('NEED', tag, ','.join(sorted(atlases)))
    elif mode == 'sheet':
        out, pattern = sys.argv[3], re.compile(sys.argv[4])
        items = [(n, sp.image(n)) for n in sorted(sp.index) if pattern.search(n)]
        items = [(n, im) for n, im in items if im is not None]
        cell, cols = 200, 6
        rows = max(1, (len(items) + cols - 1) // cols)
        sheet = Image.new('RGBA', (cols * cell, rows * (cell + 16)), (18, 16, 14, 255))
        draw = ImageDraw.Draw(sheet)
        for i, (n, im) in enumerate(items):
            t = im.copy()
            t.thumbnail((cell - 8, cell - 8))
            cx, cy = (i % cols) * cell, (i // cols) * (cell + 16)
            sheet.alpha_composite(t, (cx + (cell - t.width) // 2, cy + (cell - t.height) // 2))
            draw.text((cx + 3, cy + cell + 1), n[-22:], fill=(220, 200, 160, 255))
        sheet.save(out)
        print(len(items), 'sprites ->', out)


if __name__ == '__main__':
    main()
