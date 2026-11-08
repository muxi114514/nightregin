"""v3 素材准备：从 GA 图集复制按键图 / 框图 / 标签图标，生成失败图标、无光点细线、去光点面板，
并把 Tab_Cursor 的青绿宝石改成古铜色。可重复运行，结果一致。用法：python prep_assets.py <scratchpad>"""
import colorsys, glob, os, shutil, sys
from PIL import Image

S = sys.argv[1]
V2, V3 = os.path.join(S, 'mockups', 'v2-A-title'), os.path.join(S, 'mockups', 'v3-A-title')
G = os.path.join(S, 'ga_art', 'png')
A = os.path.join(V3, 'assets')
if not os.path.exists(V3):
    shutil.copytree(V2, V3)
    for f in os.listdir(V3):                   # v2 的截图不带过来
        if f.startswith('preview-'):
            os.remove(os.path.join(V3, f))
for f in ['quitgame.png', 'loading.png', 'title_base.png', 'vline.png', 'storage.png', 'line_spark.png']:
    p = os.path.join(A, f)
    if os.path.exists(p):
        os.remove(p)


def src(name):
    return glob.glob(os.path.join(G, '*', name + '.png'))[0]


COPY = {
    'key_ud': 'SB_KeyTop_02/KG_Key_Arrow_UD', 'key_lr': 'SB_KeyTop_02/KG_Key_Arrow_LR',
    'key_enter': 'SB_KeyTop/KG_Key_Enter', 'key_esc': 'SB_KeyTop/KG_Key_Esc',
    'key_q': 'SB_KeyTop/KG_Key_Q', 'key_e': 'SB_KeyTop/KG_Key_E',
    'tab_update': 'SB_Tab/MENU_Tab_34',        # 卷轴 + 羽毛笔，40px 下仍认得出
    'scroll_waku': 'SB_MainMenu/MENU_FL_scroll_waku', 'scroll_bar': 'SB_MainMenu/MENU_FL_scroll_Bar',
    'fl_finger': 'SB_MainMenu_02/MENU_FL_40187_FingerReader', 'fl_coliseum': 'SB_MainMenu_02/MENU_FL_40191_Coliseum',
    'fl_dragon': 'SB_MainMenu_02/MENU_FL_Dragon', 'fl_levelup': 'SB_MainMenu_02/MENU_FL_LevelUp',
    'fl_forge': 'SB_MainMenu_02/MENU_FL_40153_SoulForging', 'fl_arts': 'SB_MainMenu_02/MENU_FL_40168_Arts',
    'fl_runes': 'SB_MainMenu_02/MENU_FL_40152_BigRunes', 'fl_stone': 'SB_MainMenu_02/MENU_FL_40160_StoneChange',
    'fl_copy': 'SB_MainMenu_02/MENU_FL_40163_BossSoulCopy', 'fl_dresser': 'SB_MainMenu_02/MENU_FL_40165_Dresser',
    'fl_veil': 'SB_MainMenu_02/MENU_FL_40192_Veil', 'fl_scroll': 'SB_MainMenu_02/MENU_FL_40166_Pyroxene',
    'fl_toolbox': 'SB_MainMenu_02/MENU_FL_40136_ToolBox', 'fl_sleep': 'SB_MainMenu_02/MENU_FL_40169_Sleep',
    'bag': 'SB_MainMenu/MENU_FL_40174',
}
for dst, s in COPY.items():
    shutil.copyfile(os.path.join(G, s + '.png'), os.path.join(A, dst + '.png'))

# 失败图标：GA 八角空框 + 法环橙色「!」，和进行中 / 完成一眼区分
box = Image.open(os.path.join(A, 'chk_off.png')).convert('RGBA')
bang = Image.open(os.path.join(G, 'SB_MainMenu/MENU_FL_40188.png')).convert('RGBA').resize((26, 47), Image.LANCZOS)
box.alpha_composite(bang, ((74 - 26) // 2, (74 - 47) // 2))
box.save(os.path.join(A, 'chk_fail.png'))

# 开场画面的小图：从标志里裁出树冠
logo = Image.open(os.path.join(A, 'logo.webp')).convert('RGBA')
logo.crop((390, 0, 770, 380)).resize((112, 112), Image.LANCZOS).save(os.path.join(A, 'logo_mini.png'))

# 细线：取 Cursor_Top 最亮处那一列的竖向剖面，横向按渐隐曲线重画，去掉原图 x≈387 的光点
top = Image.open(src('MENU_FL_Cursor_Top')).convert('RGBA')
W, H = top.size
col = [top.getpixel((930, y)) for y in range(H)]
spark = top.crop((362, 0, 412, H))                # 原图光点，只在面板分隔线正中用一次


def line(fade, with_spark=False):
    im = Image.new('RGBA', (W, H))
    p = im.load()
    for x in range(W):
        f = fade(x / (W - 1))
        for y in range(H):
            r, g, b, a = col[y]
            p[x, y] = (r, g, b, round(a * f))
    if with_spark:
        mid = Image.new('RGBA', (W, H))
        mid.paste(spark, ((W - spark.width) // 2, 0))
        im.alpha_composite(mid)
    return im


line(lambda t: max(0.0, 1 - t) ** 1.3 * min(1.0, t * 12)).save(os.path.join(A, 'line_l.png'))   # 页名下：左亮右隐
line(lambda t: (4 * t * (1 - t)) ** 1.6).save(os.path.join(A, 'line_m.png'))                     # 分组、输入框：两端渐隐
line(lambda t: (4 * t * (1 - t)) ** 1.6, True).save(os.path.join(A, 'line_c.png'))               # 面板分隔线：正中一个光点

# 说明面板：Unchku 底边原生光点（x 272~466）会随面板高度上下跳，用同一行右侧的干净边框覆盖
un = Image.open(src('MENU_FL_Unchku')).convert('RGBA')
uw, uh = un.size
un.paste(un.crop((600, uh - 80, 840, uh)), (250, uh - 80))
un.save(os.path.join(A, 'unchku.png'))

# Tab_Cursor：只把青绿 / 黄绿色相（50°~235°）的像素转到古铜，饱和度约原来的 55%、明度降一档，
# 避免和警示橙（--err、「!」）撞色；其余像素原样保留
H_LO, H_HI, S_MIN = 50, 235, 0.12
NH, S_MUL, V_MUL = 30, 0.55, 0.78
cur = Image.open(src('MENU_FL_Tab_Cursor')).convert('RGBA')
px = cur.load(); cw, ch = cur.size; changed = 0
for y in range(ch):
    for x in range(cw):
        r, g, b, a = px[x, y]
        if a == 0:
            continue
        hh, ss, vv = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
        if H_LO / 360 <= hh <= H_HI / 360 and ss > S_MIN:
            nr, ng, nb = colorsys.hsv_to_rgb(NH / 360, ss * S_MUL, vv * V_MUL)
            px[x, y] = (round(nr * 255), round(ng * 255), round(nb * 255), a)
            changed += 1
cur.save(os.path.join(A, 'tab_cursor.png'))
print('tab_cursor recolored px:', changed)
tot = sum(os.path.getsize(os.path.join(A, f)) for f in os.listdir(A) if os.path.isfile(os.path.join(A, f)))
print('assets total bytes:', tot, 'files:', len(os.listdir(A)))
