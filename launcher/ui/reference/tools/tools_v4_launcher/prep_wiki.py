"""图鉴素材：复制 wiki_sample.js 与条目图片，从 GA 图集取分类标签图标、物品栏光标、地形示意图与按键图。
可重复运行。用法：python -I prep_wiki.py <GA png 目录> <wiki_data 目录> <v4 assets 目录>"""
import os, shutil, sys
from PIL import Image

G, WD, A = sys.argv[1:4]
os.makedirs(os.path.join(A, 'wiki'), exist_ok=True)
shutil.copyfile(os.path.join(WD, 'wiki_sample.js'), os.path.join(A, 'wiki_sample.js'))
n = 0
for f in os.listdir(os.path.join(WD, 'img')):
    if f.endswith('.webp'):
        shutil.copyfile(os.path.join(WD, 'img', f), os.path.join(A, 'wiki', f))
        n += 1

SPR = {
    # 分类标签：法环物品栏 / 系统菜单自带的标签图标
    'wt_weapon': 'SB_Tab/MENU_Tab_Weapon', 'wt_talisman': 'SB_Tab/MENU_Tab_15', 'wt_spell': 'SB_Tab/MENU_Tab_30_PyroxeneMagic',
    'wt_map': 'SB_Tab/MENU_Tab_04', 'wt_enemy': 'SB_Tab/MENU_Tab_36', 'wt_system': 'SB_Tab/MENU_Tab_18',
    # 特异地形没有游戏内预览图，借法环地图标签的浮雕图作示意
    'wm_volcano': 'SB_Tab_02/MENU_Tab_40385_Volcano', 'wm_snowup': 'SB_Tab_02/MENU_Tab_40386_SnowUP',
    'wm_rotten': 'SB_Tab_02/MENU_Tab_40382_RottenTree', 'wm_nokron': 'SB_Tab_02/MENU_Tab_40378_UngroPlains',
    'key_f': 'SB_KeyTop/KG_Key_F', 'key_bs': 'SB_KeyTop/KG_Key_BackSpace',
}
for dst, s in SPR.items():
    shutil.copyfile(os.path.join(G, s + '.png'), os.path.join(A, dst + '.png'))
# 物品栏选中框 MENU_SL_Cursor 原图 320px，格子只有 80px，缩到 176px 省体积
Image.open(os.path.join(G, 'SB_MainMenu/MENU_SL_Cursor.png')).convert('RGBA').resize((176, 177), Image.LANCZOS).save(os.path.join(A, 'sl_cursor.png'), optimize=True)
print('wiki images', n, '; sprites', len(SPR) + 1)
