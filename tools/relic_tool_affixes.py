"""把 MMV 整合包的自定义遗物词条（relics\\src\\content\\）加进 NightreignRelicTool 的词条库 app/data/relics.json。

- 其余字段照抄原版模板词条（兼容性、分组、可选列表、safe 等），名字 / 说明换成自定义的
- ro（遗物上词条的排列顺序）= 游戏排序键 (AttachEffectParam.overrideEffectId, ID) 的名次；自定义词条复制自模板，
  排序键与模板相同、ID 更大且中间没有其他原版词条，所以紧跟模板；同一模板的多个自定义词条按 ID 依次排列，后面的名次依次 +1
- 重复运行会先撤掉上次加的条目再重加；首次运行备份为 relics.json.bak；写回保持原格式（indent=1、CRLF）
用法：python relic_tool_affixes.py [NightreignRelicTool 目录]
"""
import copy
import json
import os
import shutil
import sys

DEFAULT_TOOL_DIR = r"D:\relink\114514\NightreignRelicTool"
NOTE = "MMV 整合包自定义词条，只在装有 StatAlloc\\pkg 的整合包里有效。"
CUSTOM = [
    {
        "id": 7010550, "template": 7010500,
        "name": "【追踪者】发动绝招时，能燃起灵火、蔓延周围",
        "nameEn": "[Wylder] Art activation spreads ghostflame in area",
        "aliases": [],
        "explanation": "发动绝招时向周围散开12团灵火，持续10秒，造成魔力伤害并积累冻伤。" + NOTE,
    },
    {
        "id": 7020050, "template": 7020000,
        "name": "【追踪者】发动技艺时，轻攻击能使出 缠绕灵火的追加攻击（仅限大剑）",
        "nameEn": "[Wylder] Standard attacks enhanced with ghostflame follow-ups when using Character Skill (greatsword only)",
        "aliases": ["【追踪者】发动技艺时，轻攻击能使出缠绕灵火的追加攻击（仅限大剑）", "【追踪者】发动技艺时，能使出灵火追加攻击(仅限大剑)"],
        "explanation": "使用技艺后用大剑进行轻攻击，武器附加灵火（魔力属性攻击力+25，命中积累冻伤），持续20秒。" + NOTE,
    },
]
# 血焰 / 黑焰 / 命定之死版（relics\src\content\wylder_elements.rs）：(追加攻击 ID, 绝招 ID, 中文名, 地火中文名, 英文名, 地火英文名, 附魔说明, 地火说明)
ELEMENTS = [
    (7020060, 7010570, "血焰", "血焰", "bloodflame", "bloodflame",
     "火属性攻击力+20，命中积累出血", "造成火属性伤害并积累出血"),
    (7020070, 7010580, "黑焰", "黑焰", "black flame", "black flame",
     "火属性攻击力+20，命中附加黑焰（按血量上限比例的持续伤害）", "造成火属性伤害并附加黑焰"),
    (7020080, 7010590, "命定之死", "命定之死的火焰", "Destined Death", "Destined Death flames",
     "圣属性攻击力+20，命中附加命定之死（持续伤害、降低血量上限）", "造成火属性伤害并附加命定之死"),
]
for follow_up, ultimate, zh, zh_flame, en, en_flame, enchant, ground in ELEMENTS:
    CUSTOM += [
        {
            "id": ultimate, "template": 7010500,
            "name": f"【追踪者】发动绝招时，能燃起{zh_flame}、蔓延周围",
            "nameEn": f"[Wylder] Art activation spreads {en_flame} in area",
            "aliases": [],
            "explanation": f"发动绝招时向周围散开12团{zh_flame}，持续10秒，{ground}。" + NOTE,
        },
        {
            "id": follow_up, "template": 7020000,
            "name": f"【追踪者】发动技艺时，轻攻击能使出 缠绕{zh}的追加攻击（仅限大剑）",
            "nameEn": f"[Wylder] Standard attacks enhanced with {en} follow-ups when using Character Skill (greatsword only)",
            "aliases": [f"【追踪者】发动技艺时，轻攻击能使出缠绕{zh}的追加攻击（仅限大剑）", f"【追踪者】发动技艺时，能使出{zh}追加攻击(仅限大剑)"],
            "explanation": f"使用技艺后用大剑进行轻攻击，武器附加{zh}（{enchant}），持续20秒。" + NOTE,
        },
    ]


def shift_ro(affixes, start, delta):
    for a in affixes:
        if a["ro"] is not None and a["ro"] >= start:
            a["ro"] += delta


def remove_custom(affixes):
    ids = {c["id"] for c in CUSTOM}
    for a in [a for a in affixes if a["id"] in ids]:
        affixes.remove(a)
        if a["ro"] is not None:
            shift_ro(affixes, a["ro"] + 1, -1)


def add_custom(affixes):
    for c in sorted(CUSTOM, key=lambda c: c["id"]):
        index = next(i for i, a in enumerate(affixes) if a["id"] == c["template"])
        template = affixes[index]
        # 跳过同一模板下已插入、ID 更小的自定义词条，保持按 ID 排列
        while index + 1 < len(affixes) and affixes[index + 1].get("_template") == c["template"]:
            index += 1
        prev = affixes[index]
        entry = copy.deepcopy(template)
        entry.update({k: v for k, v in c.items() if k != "template"})
        entry["_template"] = c["template"]
        if prev["ro"] is not None:
            entry["ro"] = prev["ro"] + 1
            shift_ro(affixes, entry["ro"], 1)
        affixes.insert(index + 1, entry)
    for a in affixes:
        a.pop("_template", None)


def recount(meta, affixes):
    lists = lambda name: sum(1 for a in affixes if name in a["lists"])
    meta["counts"] = {
        "total": len(affixes),
        "safe": sum(1 for a in affixes if a["safe"]),
        "illegal": sum(1 for a in affixes if a["group"] == "ILLEGAL"),
        "std": lists("RelicIDStd"),
        "don": lists("RelicIDDoN"),
        "debuff": lists("RelicIDDebuff"),
        "orderable": sum(1 for a in affixes if a["ro"] is not None),
    }


def main():
    tool_dir = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_TOOL_DIR
    path = os.path.join(tool_dir, "app", "data", "relics.json")
    with open(path, "rb") as f:
        db = json.loads(f.read().decode("utf-8"))
    affixes = db["affixes"]
    before = [a["id"] for a in sorted((a for a in affixes if a["ro"] is not None), key=lambda a: a["ro"])]

    remove_custom(affixes)
    add_custom(affixes)
    recount(db["meta"], affixes)

    ros = sorted(a["ro"] for a in affixes if a["ro"] is not None)
    if ros != list(range(ros[0], ros[0] + len(ros))):
        raise SystemExit("ro 不再是连续名次，未写入")
    custom_ids = {c["id"] for c in CUSTOM}
    after = [a["id"] for a in sorted((a for a in affixes if a["ro"] is not None), key=lambda a: a["ro"])]
    if [i for i in after if i not in custom_ids] != [i for i in before if i not in custom_ids]:
        raise SystemExit("原有词条的相对顺序被改变，未写入")

    backup = path + ".bak"
    if not os.path.exists(backup):
        shutil.copy2(path, backup)
    text = json.dumps(db, ensure_ascii=False, indent=1).replace("\n", "\r\n")
    with open(path, "wb") as f:
        f.write(text.encode("utf-8"))
    for c in CUSTOM:
        a = next(a for a in affixes if a["id"] == c["id"])
        print(f"{a['id']} ro={a['ro']} {a['compat']} safe={a['safe']} {a['name']}")
    print(f"已写入 {path}（共 {len(affixes)} 条）")


if __name__ == "__main__":
    main()
