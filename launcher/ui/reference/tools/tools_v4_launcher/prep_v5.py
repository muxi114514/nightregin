"""第五轮素材：裁掉 Unchku_s 透明边作物品格（格框相接）、取法环新物品标记、大空洞示意浮雕。
用法：python -I prep_v5.py <GA png 目录> <v4 assets 目录>"""
import os, sys
from PIL import Image

G, A = sys.argv[1:3]
im = Image.open(os.path.join(A, 'unchku_s.png')).convert('RGBA')
box = im.getchannel('A').point(lambda v: 255 if v > 40 else 0).getbbox()
slot = im.crop(box)
slot.resize((200, 200), Image.LANCZOS).save(os.path.join(A, 'slot.png'), optimize=True)
nm = Image.open(os.path.join(G, 'SB_KeyTop_02/KG_TEX_NewItem_mark.png')).convert('RGBA')
nm.crop(nm.getchannel('A').getbbox()).save(os.path.join(A, 'newmark.png'), optimize=True)
Image.open(os.path.join(G, 'SB_Tab_02/MENU_Tab_40380_DownhillRuins.png')).convert('RGBA').save(os.path.join(A, 'wm_hollow.png'), optimize=True)
print('slot crop', box)
