"""把 ga_art_dump.ps1 导出的 GA 图集裁成单张 PNG，供启动器草图使用（只读原文件）。
用法：python -I ga_art_png.py <导出目录> <NRStatAlloc\\tools 目录>
产物：<导出目录>\\png\\<图集>\\<子图>.png、logo_ga.png / logo_nightreign.png、ember.webp（60 帧循环）、ember_still.png、contact_*.png
"""
import io
import os
import re
import struct
import sys

from PIL import Image, ImageDraw

ROOT, TOOLS = sys.argv[1], sys.argv[2]
sys.path.insert(0, TOOLS)
import ga_title_logo  # noqa: E402  用户自己的标志改字工具

BLOCK8 = {71, 72, 80, 81}
SPRITE_RE = re.compile(r'<SubTexture name="([^"]+)" x="(\d+)" width="(\d+)" y="(\d+)" height="(\d+)"')
ALT_RE = re.compile(r'<SubTexture name="([^"]+)"[^>]*?x="(\d+)"[^>]*?y="(\d+)"[^>]*?width="(\d+)"[^>]*?height="(\d+)"')


def dds_info(raw):
    h, w = struct.unpack_from('<II', raw, 12)
    if raw[84:88] == b'DX10':
        fmt = struct.unpack_from('<I', raw, 128)[0]
        return w, h, 148, (8 if fmt in BLOCK8 else 16)
    return w, h, 128, (8 if raw[84:88] == b'DXT1' else 16)


def crop(raw, x, y, w, h):
    """按 4x4 块对齐向外取整后裁出、解码，再精确裁到目标矩形"""
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


def sprites(layout_path):
    text = open(layout_path, 'rb').read().decode('utf-8', 'replace')
    found = [(m.group(1), int(m.group(2)), int(m.group(4)), int(m.group(3)), int(m.group(5))) for m in SPRITE_RE.finditer(text)]
    if not found:
        found = [(m.group(1), int(m.group(2)), int(m.group(3)), int(m.group(4)), int(m.group(5))) for m in ALT_RE.finditer(text)]
    return found  # (name, x, y, w, h)


def contact(images, out_path, cell=160, cols=10):
    rows = (len(images) + cols - 1) // cols
    sheet = Image.new('RGBA', (cols * cell, rows * (cell + 18)), (18, 16, 14, 255))
    draw = ImageDraw.Draw(sheet)
    for i, (name, img) in enumerate(images):
        thumb = img.copy()
        thumb.thumbnail((cell - 8, cell - 8))
        cx, cy = (i % cols) * cell, (i // cols) * (cell + 18)
        sheet.alpha_composite(thumb, (cx + (cell - thumb.width) // 2, cy + (cell - thumb.height) // 2))
        draw.text((cx + 3, cy + cell + 2), name[:24], fill=(200, 190, 170, 255))
    sheet.save(out_path)


def main():
    png_root = os.path.join(ROOT, 'png')
    atlases = {
        'SB_Title_01': None, 'SB_Title': None, 'SB_MainMenu': None, 'SB_MainMenu_02': None,
        'SB_MainMenu_03': None, 'SB_Tab': None, 'SB_Tab_02': None, 'SB_GA_MainMenu': None,
        'SB_KeyTop': None, 'SB_KeyTop_02': None,
    }
    only = set(os.environ.get('GA_ART_ONLY', '').split(',')) - {''}
    if only:
        atlases = {k: v for k, v in atlases.items() if k in only}
    for atlas in atlases:
        dds_path = os.path.join(ROOT, 'dds', atlas + '.dds')
        lay_path = os.path.join(ROOT, 'layouts', atlas + '.layout')
        if not (os.path.exists(dds_path) and os.path.exists(lay_path)):
            print('skip', atlas)
            continue
        raw = open(dds_path, 'rb').read()
        out_dir = os.path.join(png_root, atlas)
        os.makedirs(out_dir, exist_ok=True)
        items = []
        for name, x, y, w, h in sprites(lay_path):
            img = crop(raw, x, y, w, h)
            stem = os.path.splitext(name)[0]
            img.save(os.path.join(out_dir, stem + '.png'))
            items.append((stem, img))
        contact(items, os.path.join(ROOT, f'contact_{atlas}.png'))
        print(atlas, len(items), 'sprites')
    if only:
        return

    # 标志：GA 原图与换成 NIGHTREIGN 的版本
    t1 = open(os.path.join(ROOT, 'dds', 'SB_Title_01.dds'), 'rb').read()
    logo = crop(t1, 0, 60, 2532, 1532)
    logo.save(os.path.join(ROOT, 'logo_ga.png'))
    ga_title_logo.relabel(logo).save(os.path.join(ROOT, 'logo_nightreign.png'))
    print('logo done')

    # 余烬：60 格 1920x1080，缩到 960x540 做循环动图（每格约 33ms）
    t2 = open(os.path.join(ROOT, 'dds', 'SB_Title_02.dds'), 'rb').read()
    frames = []
    for r in range(12):
        for c in range(5):
            frames.append(crop(t2, c * 1920, r * 1080, 1920, 1080).resize((960, 540), Image.LANCZOS))
    frames[30].save(os.path.join(ROOT, 'ember_still.png'))
    frames[0].save(os.path.join(ROOT, 'ember.webp'), save_all=True, append_images=frames[1:],
                   duration=33, loop=0, quality=82, method=4)
    alpha = frames[30].getchannel('A').getextrema()
    print('ember done, alpha range', alpha, 'size', os.path.getsize(os.path.join(ROOT, 'ember.webp')))


if __name__ == '__main__':
    main()
