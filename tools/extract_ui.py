"""从游戏菜单图集裁出属性分配面板用的原生素材，存成 PNG（由 extract_ui.ps1 调用，需要 Pillow）。

用法：python -I extract_ui.py <图集目录(dds + layout)> <输出目录>
输出文件名由 DLL 的 src/ui/skin.rs 读取，两边需保持一致；武器图标存到 icons\<5 位图标编号>.png。
"""
import re
import sys
from pathlib import Path

from PIL import Image

# 输出名 -> (图集, 子图名, 最长边上限；None 为原尺寸)
SPRITES = {
    "background": ("SB_Common_1", "MENU_Cmn_bg_06", None),
    "panel_frame": ("SB_In_Game_0", "MENU_In_bg_17", None),
    "panel_fill": ("SB_In_Game_0", "MENU_In_bg_12", 256),
    "button": ("SB_In_Game_0", "MENU_In_bg_21", None),
    "button_hot": ("SB_In_Game_0", "MENU_In_Cursor_07", None),
    "row_highlight": ("SB_In_Game_1", "MENU_In_Cursor_19", None),
    "arrow": ("SB_In_Game_0", "MENU_In_Arrow_04", None),
    "line": ("SB_In_Game_1", "MENU_In_Line_00", None),
    "title_glow": ("SB_In_Game_1", "MENU_In_Title_00", None),
    "diamond_on": ("SB_In_Game_0", "MENU_In_Circle_01", None),
    "diamond_off": ("SB_In_Game_0", "MENU_In_Circle_00", None),
    "key_updown": ("SB_KeyTop_02_0", "KG_Key_Arrow_UD", None),
    "key_leftright": ("SB_KeyTop_02_0", "KG_Key_Arrow_LR", None),
    "key_q": ("SB_KeyTop_0", "KG_Key_Q", None),
    "key_e": ("SB_KeyTop_0", "KG_Key_E", None),
    "key_enter": ("SB_KeyTop_0", "KG_Key_Enter", None),
    "key_esc": ("SB_KeyTop_0", "KG_Key_Esc", None),
}

# 10 个角色 13 把初始武器的图标编号（EquipParamWeapon.iconId），在三张物品图标图集里
ICONS = [10017, 10102, 10167, 10329, 10415, 10697, 10813, 10896, 11048, 11220, 11240, 12021, 12109]
ICON_ATLASES = ["SB_ItemIcon_1", "SB_ItemIcon_2", "SB_ItemIcon_DLC01_0"]
# 遗物图标（EquipParamAntique.iconId）：191xx~193xx 各颜色 / 大小的情景与暗淡情景、1995x，以及固定词条遗物的 416 / 428；
# 同图集里 190xx 是器皿，不导出。重铸面板按约 80 像素显示，缩到 128 以内
RELIC_ICON = re.compile(r"MENU_ItemIcon_(19[1-3]\d\d|1995\d|00416|00428)$")
RELIC_ATLASES = ["SB_ItemIcon_0", "SB_ItemIcon_2", "SB_ItemIcon_DLC01_0"]
RELIC_ICON_LIMIT = 128
# 全部武器图标（词条继承面板按需加载）：EquipParamWeapon.iconId 绝大多数在 10000~12999，MMV 另有 52xxx，空城 0.46 另有 22xxx；
# 同样缩到 128 以内，初始武器图标随后按原尺寸覆盖。MMV 的新增武器图标在它自己的几张图集里（extract_ui.ps1 存在才导出）
WEAPON_ICON = re.compile(r"MENU_ItemIcon_(1[0-2]\d{3}|22\d{3}|52\d{3})$")
OPTIONAL_WEAPON_ATLASES = ["MoreWeapons", "MMV", "jem"]

# 子图：属性顺序因图集而异（MMV 的是 x y width height），按属性名取
SUB = re.compile(r"<SubTexture\s([^>]*)/?>")
ATTR = re.compile(r'(\w+)="([^"]*)"')


def load(src: Path, atlases: dict, atlas: str):
    """图集图像与子图矩形（按名缓存）"""
    if atlas not in atlases:
        layout = (src / f"{atlas}.layout").read_text(encoding="utf-8", errors="replace")
        rects = {}
        for tag in SUB.findall(layout):
            a = dict(ATTR.findall(tag))
            name = re.sub(r"\.(png|dds)$", "", a.get("name", ""))
            if name and all(k in a for k in ("x", "y", "width", "height")):
                rects[name] = tuple(int(a[k]) for k in ("x", "y", "width", "height"))
        atlases[atlas] = (Image.open(src / f"{atlas}.dds").convert("RGBA"), rects)
    return atlases[atlas]


def crop(image, rect, limit=None):
    x, y, w, h = rect
    part = image.crop((x, y, x + w, y + h))
    if limit and max(w, h) > limit:
        scale = limit / max(w, h)
        part = part.resize((max(1, round(w * scale)), max(1, round(h * scale))), Image.LANCZOS)
    return part


def main(src: Path, out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    atlases = {}
    for name, (atlas, sprite, limit) in SPRITES.items():
        image, rects = load(src, atlases, atlas)
        if sprite not in rects:
            raise SystemExit(f"{sprite} not found in {atlas}")
        part = crop(image, rects[sprite], limit)
        part.save(out / f"{name}.png", optimize=True)
        print(f"{name}.png  {sprite}  {part.size[0]}x{part.size[1]}")
    (out / "icons").mkdir(exist_ok=True)
    weapons = 0
    for atlas in ICON_ATLASES + [a for a in OPTIONAL_WEAPON_ATLASES if (src / f"{a}.dds").exists()]:
        image, rects = load(src, atlases, atlas)
        for sprite, rect in rects.items():
            m = WEAPON_ICON.match(sprite)
            if m:
                crop(image, rect, RELIC_ICON_LIMIT).save(out / "icons" / f"{int(m.group(1)):05d}.png", optimize=True)
                weapons += 1
    print(f"weapon icons: {weapons}")
    for icon in ICONS:
        sprite = f"MENU_ItemIcon_{icon:05d}"
        found = next((a for a in ICON_ATLASES if sprite in load(src, atlases, a)[1]), None)
        if found is None:
            raise SystemExit(f"{sprite} not found in {ICON_ATLASES}")
        image, rects = atlases[found]
        crop(image, rects[sprite]).save(out / "icons" / f"{icon:05d}.png", optimize=True)
        print(f"icons/{icon:05d}.png  {found}")
    relics = 0
    for atlas in RELIC_ATLASES:
        image, rects = load(src, atlases, atlas)
        for sprite, rect in rects.items():
            m = RELIC_ICON.match(sprite)
            if m:
                crop(image, rect, RELIC_ICON_LIMIT).save(out / "icons" / f"{int(m.group(1)):05d}.png", optimize=True)
                relics += 1
    print(f"relic icons: {relics}")


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]))
