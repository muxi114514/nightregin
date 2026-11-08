"""开场界面换成 GoldenAge 的：黄金树标志 + 余烬背景（由 tools\\ga_title.ps1 调用，要 Pillow + numpy）。

  atlas <GA SB_Title_01.dds> <GA SB_Title_02.dds> <参考布局> <输出.dds> <输出.layout>
      余烬是 60 格 1920x1080 的序列图（9600x12960），缩到一半存（布局 half="1"，界面文件仍按原尺寸显示）；
      标志取 SB_Title_01 里的 MENU_Title_EldenRing_01，「ELDEN RING」换成「NIGHT REIGN」（ga_title_logo.py）。两张拼进一张 DXT5 图集
  gfx <黑夜君临 05_001_title_logo.gfx> <GA 05_001_title_logo.gfx> <输出.gfx>
      保留黑夜君临的流程、脚本和背景视频；在视频层和标题层之间插入 GA 的余烬剪辑，淡入淡出跟视频层一致；
      标题层换成 GA 标志，用 GA 的缩放
"""
import io
import os
import struct
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gfx_swf as g
import ga_title_logo

EMBER_NAME = 'MENU_GA_TitleEmber'
LOGO_NAME = 'MENU_GA_TitleLogo'
ATLAS_NAME = 'SB_GA_Title_0'
CELL_W, CELL_H = 1920, 1080
CELL_COLS, CELL_ROWS = 5, 12
EMBER_W, EMBER_H = CELL_W * CELL_COLS, CELL_H * CELL_ROWS
# GA 的标志在 SB_Title_01 里的位置
LOGO_RECT = (0, 60, 2532, 1532)
HALF = 2
# 黑夜君临开场：视频在深度 3，标题图在深度 5；余烬插在 4
MOVIE_DEPTH = 3
TITLE_DEPTH = 5
EMBER_DEPTH = 4
# GA 里余烬剪辑和标志用的号
GA_EMBER_IMAGE = 666
GA_EMBER_CLIP = 667
GA_LOGO_IMAGE = 4
BLOCK8 = {71, 72, 80, 81}


def dds_info(raw):
    h, w = struct.unpack_from('<II', raw, 12)
    if raw[84:88] == b'DX10':
        fmt = struct.unpack_from('<I', raw, 128)[0]
        return w, h, 148, (8 if fmt in BLOCK8 else 16)
    return w, h, 128, (8 if raw[84:88] == b'DXT1' else 16)


def crop(raw, x, y, w, h):
    """按 4x4 块裁出一块再解码，大图不用整张展开"""
    width, _, head, block = dds_info(raw)
    row = (width + 3) // 4
    out = bytearray()
    for by in range(y // 4, (y + h) // 4):
        at = head + (by * row + x // 4) * block
        out += raw[at:at + (w // 4) * block]
    hdr = bytearray(raw[:head])
    struct.pack_into('<II', hdr, 12, h, w)
    struct.pack_into('<I', hdr, 20, len(out) // (h // 4))
    struct.pack_into('<I', hdr, 28, 1)
    return Image.open(io.BytesIO(bytes(hdr) + bytes(out))).convert('RGBA')


def build_atlas(title01, title02, reference, out_dds, out_layout):
    t1 = open(title01, 'rb').read()
    t2 = open(title02, 'rb').read()
    w2, h2, _, _ = dds_info(t2)
    if (w2, h2) != (EMBER_W, EMBER_H):
        raise ValueError(f'ember sheet is {w2}x{h2}, expected {EMBER_W}x{EMBER_H}')
    ew, eh = EMBER_W // HALF, EMBER_H // HALF
    lx, ly, lw, lh = LOGO_RECT
    width = ew
    height = eh + (lh + 3) // 4 * 4
    atlas = Image.new('RGBA', (width, height), (0, 0, 0, 0))
    cw, ch = CELL_W // HALF, CELL_H // HALF
    for r in range(CELL_ROWS):
        for c in range(CELL_COLS):
            cell = crop(t2, c * CELL_W, r * CELL_H, CELL_W, CELL_H).resize((cw, ch), Image.LANCZOS)
            atlas.paste(cell, (c * cw, r * ch))
    atlas.paste(ga_title_logo.relabel(crop(t1, lx, ly, lw, lh)), (0, eh))
    atlas.save(out_dds, 'DDS', pixel_format='DXT5')
    ref = open(reference, 'rb').read().decode('utf-8')
    newline = '\r\n' if '\r\n' in ref else '\n'
    path = ref.split('imagePath="', 1)[1].split('"', 1)[0]
    folder = path[:path.rfind('\\') + 1]
    lines = [
        f'<TextureAtlas imagePath="{folder}{ATLAS_NAME}.tif" width="{width}" height="{height}">',
        f'\t<SubTexture name="{EMBER_NAME}.png" x="0" width="{ew}" y="0" height="{eh}" originalWidth="{EMBER_W}" originalHeight="{EMBER_H}" half="1"/>',
        f'\t<SubTexture name="{LOGO_NAME}.png" x="0" width="{lw}" y="{eh}" height="{lh}" originalWidth="{lw}" originalHeight="{lh}" half="0"/>',
        '</TextureAtlas>',
    ]
    open(out_layout, 'wb').write((newline.join(lines) + newline).encode('utf-8'))
    print(f'图集 {width}x{height}，余烬 {ew}x{eh}（半尺寸），标志 {lw}x{lh}')


def set_char(tag, char):
    """改摆放标签里的角色号（不动其它字段）"""
    pl = g.decode_place(tag)
    if 'char' not in pl:
        raise ValueError('placement has no character')
    body = bytearray(tag.body)
    at = (1 if tag.code == g.TAG_PLACE2 else 2) + 2
    if tag.code == g.TAG_PLACE3 and pl['flags2'] & 0x08:
        at = tag.body.index(0, at) + 1
    struct.pack_into('<H', body, at, char)
    return g.Tag(tag.code, body, tag.long_form)


def set_place3(tag, char, matrix):
    pl = g.decode_place(tag)
    if tag.code != g.TAG_PLACE3 or pl['flags'] != 0x06 or pl['flags2'] != 0x10 or pl['rest']:
        raise ValueError('unexpected title placement layout')
    body = bytes((0x06, 0x10)) + struct.pack('<HH', pl['depth'], char) + g.write_matrix(matrix)
    return g.Tag(tag.code, body, tag.long_form)


def build_gfx(nr_path, ga_path, out_path):
    nr = g.Gfx(open(nr_path, 'rb').read())
    ga = g.Gfx(open(ga_path, 'rb').read())
    names = {g.external_image(t)['name'] for t in nr.tags if t.code == g.TAG_EXTERNAL_IMAGE}
    if EMBER_NAME in names:
        raise ValueError('title already converted')
    # GA 的余烬剪辑和标志的摆放矩阵
    ember_clip = logo_matrix = None
    for t in ga.tags:
        if t.code == g.TAG_SPRITE:
            sid = struct.unpack_from('<H', t.body, 0)[0]
            subs = g.parse_tags(t.body, 4, len(t.body))
            if sid == GA_EMBER_CLIP:
                ember_clip = (struct.unpack_from('<H', t.body, 2)[0], subs)
            for s in subs:
                if s.code in (g.TAG_PLACE2, g.TAG_PLACE3) and g.decode_place(s).get('char') == GA_LOGO_IMAGE:
                    logo_matrix = g.decode_place(s)['matrix']
    if not ember_clip or not logo_matrix:
        raise ValueError('GA ember clip or logo placement not found')
    base = nr.max_char() + 1
    ember_img, logo_img, clip_id = base, base + 1, base + 2
    frames, subs = ember_clip
    clip_tags = []
    for s in subs:
        if s.code in (g.TAG_PLACE2, g.TAG_PLACE3) and g.decode_place(s).get('char') == GA_EMBER_IMAGE:
            s = set_char(s, ember_img)
        elif s.code in (g.TAG_PLACE2, g.TAG_PLACE3) and 'char' in g.decode_place(s):
            raise ValueError('ember clip places an unexpected character')
        clip_tags.append(s)
    clip_body = struct.pack('<HH', clip_id, frames) + b''.join(s.encode() for s in clip_tags)
    defs = [g.make_external_image(ember_img, EMBER_W, EMBER_H, EMBER_NAME), g.make_external_image(logo_img, LOGO_RECT[2], LOGO_RECT[3], LOGO_NAME),
            g.Tag(g.TAG_SPRITE, clip_body, True)]
    out = []
    replaced = mirrored = 0
    for t in nr.tags:
        if t.code in (g.TAG_PLACE2, g.TAG_PLACE3):
            pl = g.decode_place(t)
            if pl['depth'] == TITLE_DEPTH and 'char' in pl:
                out += defs
                out.append(g.make_place2(EMBER_DEPTH, clip_id, [1.0, 1.0, 0.0, 0.0, 0, 0]))
                t = set_place3(t, logo_img, logo_matrix)
                replaced += 1
            elif pl['depth'] == MOVIE_DEPTH and pl['flags'] & 0x01 and 'char' not in pl:
                out.append(t)
                twin = bytearray(t.body)
                struct.pack_into('<H', twin, 1 if t.code == g.TAG_PLACE2 else 2, EMBER_DEPTH)
                out.append(g.Tag(t.code, twin, t.long_form))
                mirrored += 1
                continue
        elif t.code == g.TAG_REMOVE2 and struct.unpack_from('<H', t.body, 0)[0] == MOVIE_DEPTH:
            raise ValueError('movie layer is removed; mirror it too')
        out.append(t)
    if replaced != 1 or mirrored < 10:
        raise ValueError(f'unexpected timeline: title placements {replaced}, movie moves {mirrored}')
    nr.tags = out
    data = nr.encode()
    check = g.Gfx(data)
    if check.encode() != data:
        raise ValueError('round trip of the result differs')
    open(out_path, 'wb').write(data)
    print(f'开场界面：余烬剪辑 {clip_id}（{frames} 帧）插在深度 {EMBER_DEPTH}，淡入淡出 {mirrored} 处；标题层换成图片 {logo_img}，缩放 {logo_matrix[0]:.3f}')


def main():
    if len(sys.argv) == 7 and sys.argv[1] == 'atlas':
        build_atlas(*sys.argv[2:])
    elif len(sys.argv) == 5 and sys.argv[1] == 'gfx':
        build_gfx(*sys.argv[2:])
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main()
