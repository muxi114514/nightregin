"""v4 素材准备：夜王头像、深夜层数徽章、深夜行图标、按键图、物品栏光标，以及「永夜」暗金格框。
可重复运行，结果一致。用法：python prep_v4.py <GA 素材 png 目录> <v4 assets 目录>"""
import colorsys, os, shutil, sys
from PIL import Image

G, A = sys.argv[1], sys.argv[2]
LORDS = ['Gladius', 'Adel', 'Gnoster', 'Maris', 'Libra', 'Fulghor', 'Caligo']   # nighter.json 编号 0~6
COPY = {f'lord_{i}': f'SB_GA_MainMenu/SB_GA_NightBoss_{n}' for i, n in enumerate(LORDS)}
COPY.update({f'lv_{i + 1}': f'SB_GA_MainMenu/SB_GA_Level_{r}' for i, r in enumerate(['I', 'II', 'III', 'IV', 'V'])})
COPY.update({
    'fl_renara': 'SB_MainMenu_02/MENU_FL_40164_Renara',     # 托着圆月的手：强制深夜
    'fl_buddy': 'SB_MainMenu_02/MENU_FL_40162_Buddy',       # 门龛里的人：解锁深夜
    'key_udlr': 'SB_KeyTop_02/KG_Key_Arrow_UDLR', 'key_r': 'SB_KeyTop/KG_Key_R',
    'sel_cursor': 'SB_MainMenu/MENU_FL_Select_Cursor',      # 法环物品栏的格子光标
})
for dst, s in COPY.items():
    shutil.copyfile(os.path.join(G, s + '.png'), os.path.join(A, dst + '.png'))

# 「永夜」格框：SlotBase 的亮橙边转成暗金（色相 38°、饱和度 ×0.5、明度 ×0.62），内里压暗，不做成霓虹
H_NEW, S_MUL, V_MUL, IN_MUL = 38, 0.5, 0.62, 0.55
im = Image.open(os.path.join(G, 'SB_MainMenu/MENU_SlotBase.png')).convert('RGBA')
px = im.load(); w, h = im.size
for y in range(h):
    for x in range(w):
        r, g, b, a = px[x, y]
        if a == 0:
            continue
        hh, ss, vv = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
        if ss > 0.25:                                  # 橙金描边
            nr, ng, nb = colorsys.hsv_to_rgb(H_NEW / 360, ss * S_MUL, vv * V_MUL)
        else:                                          # 内里石纹
            nr, ng, nb = r / 255 * IN_MUL, g / 255 * IN_MUL, b / 255 * IN_MUL
        px[x, y] = (round(nr * 255), round(ng * 255), round(nb * 255), a)
im.resize((168, 170), Image.LANCZOS).save(os.path.join(A, 'slot_lord.png'))
print('copied', len(COPY), 'files; slot_lord', im.size)
