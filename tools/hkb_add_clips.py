"""往行为图（chr\\c0000.behbnd 的 Behaviors\\c0000.hkx，Havok TAG0）新增招式表的动画片段。
行为图按「选择器 CustomManualSelectorGenerator 的动作号 + 子片段名里的招式表号」选片段，招式表没有片段就退回默认动作，
所以加新招式表除了 TAE 和动画，还要给每个动作的选择器挂一个片段。GoldenAge 的 a73 片段就是这样做的：
和同一选择器下的兄弟片段相比，只有名字、动画名、内部编号不同。

做法是只追加不改动：DATA 末尾加新对象，选择器换成新的子片段数组，其他字节都不动（旧数组留在文件里没人引用）。
用法：hkb_add_clips.py <输入.hkx> <输出.hkx> <片段清单.txt>，清单每行一个动画名（a073_030010）。
只有输入文件里已经有兄弟片段的动作才能加；没有选择器的动作会列出来。
"""
import bisect
import struct
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

SECTION_HEAD = 8
ITEM_SIZE = 12
ALIGN = 16
POINTER_SIZE = 8
SIZE_MASK = 0x3FFFFFFF
TYPE_MASK = 0xFFFFFF
# 选择器按子片段的招式表号分家族，只给同家族的选择器加片段：攻击 a20~a66，站姿 / 移动 a0x / a1x，待机姿势 a320~a365
FAMILIES = {
    "weapon": lambda s: 0 <= s <= 16 or 20 <= s <= 66,
    "idle": lambda s: 320 <= s <= 365,
}


def family_of(section):
    return "idle" if 300 <= section < 400 else "weapon"


def sections(buf, start, end):
    out = []
    p = start
    while p + SECTION_HEAD <= end:
        head = struct.unpack_from(">I", buf, p)[0]
        size = head & SIZE_MASK
        out.append((buf[p + 4:p + 8].decode("latin-1"), p, size, head & ~SIZE_MASK))
        p += size
    return out


def varint(b, p):
    x = b[p]
    if x < 0x80:
        return x, p + 1
    if x & 0xC0 == 0x80:
        return ((x << 8) | b[p + 1]) & 0x3FFF, p + 2
    if x & 0xE0 == 0xC0:
        return ((x << 16) | (b[p + 1] << 8) | b[p + 2]) & 0x1FFFFF, p + 3
    if x & 0xF8 == 0xE0:
        return int.from_bytes(b[p:p + 4], "big") & 0x7FFFFFF, p + 4
    if x & 0xF8 == 0xE8:
        return int.from_bytes(b[p:p + 5], "big") & 0x7FFFFFFFF, p + 5
    raise ValueError(f"varint {x:#x} at {p}")


def strings(b):
    return [s.decode("latin-1") for s in bytes(b).split(b"\0")]


class Types:
    """类型表：只取类型名、父类、大小和成员偏移"""

    def __init__(self, buf, section):
        sub = {s[0]: s for s in sections(buf, section[1] + SECTION_HEAD, section[1] + section[2])}
        body = lambda k: buf[sub[k][1] + SECTION_HEAD:sub[k][1] + sub[k][2]]
        type_names = strings(body("TSTR"))
        field_names = strings(body("FSTR"))
        names = body("TNA1")
        p = 0
        count, p = varint(names, p)
        self.names = {}
        for i in range(1, count):
            name, p = varint(names, p)
            templates, p = varint(names, p)
            for _ in range(templates):
                _, p = varint(names, p)
                _, p = varint(names, p)
            self.names[i] = type_names[name]
        bodies = body("TBDY")
        p = 0
        self.info = {}
        while p < len(bodies):
            tid, p = varint(bodies, p)
            if tid == 0:
                continue
            parent, p = varint(bodies, p)
            flags, p = varint(bodies, p)
            info = {"parent": parent, "members": {}, "size": None}
            if flags & 1:
                _, p = varint(bodies, p)
            if flags & 2:
                _, p = varint(bodies, p)
            if flags & 4:
                _, p = varint(bodies, p)
            if flags & 8:
                info["size"], p = varint(bodies, p)
                _, p = varint(bodies, p)
            if flags & 0x10:
                _, p = varint(bodies, p)
            if flags & 0x20:
                members, p = varint(bodies, p)
                for _ in range(members & 0xFFFF):
                    name, p = varint(bodies, p)
                    _, p = varint(bodies, p)
                    offset, p = varint(bodies, p)
                    _, p = varint(bodies, p)
                    info["members"][field_names[name]] = offset
            if flags & 0x40:
                interfaces, p = varint(bodies, p)
                for _ in range(interfaces):
                    _, p = varint(bodies, p)
                    _, p = varint(bodies, p)
            if flags & 0x80:
                _, p = varint(bodies, p)
            self.info[tid] = info

    def find(self, name):
        hits = [k for k, v in self.names.items() if v == name]
        if len(hits) != 1:
            raise ValueError(f"type {name}: {hits}")
        return hits[0]

    def fields(self, tid):
        info = self.info[tid]
        out = self.fields(info["parent"]) if info["parent"] else {}
        out.update(info["members"])
        return out


class Tagfile:
    def __init__(self, buf):
        self.buf = buf
        root = sections(buf, 0, len(buf))
        if len(root) != 1 or root[0][0] != "TAG0":
            raise ValueError("not a TAG0 file")
        self.root = root[0]
        self.secs = sections(buf, self.root[1] + SECTION_HEAD, self.root[1] + self.root[2])
        by = {s[0]: s for s in self.secs}
        self.types = Types(buf, by["TYPE"])
        data = by["DATA"]
        self.data = bytearray(buf[data[1] + SECTION_HEAD:data[1] + data[2]])
        self.index = by["INDX"]
        self.index_secs = sections(buf, self.index[1] + SECTION_HEAD, self.index[1] + self.index[2])
        sub = {s[0]: s for s in self.index_secs}
        item = sub["ITEM"]
        self.items = [list(struct.unpack_from("<III", buf, item[1] + SECTION_HEAD + i * ITEM_SIZE)) for i in range((item[2] - SECTION_HEAD) // ITEM_SIZE)]
        patch = sub["PTCH"]
        self.patches = []
        p = patch[1] + SECTION_HEAD
        while p + 8 <= patch[1] + patch[2]:
            ty, n = struct.unpack_from("<II", buf, p)
            self.patches.append([ty, list(struct.unpack_from(f"<{n}I", buf, p + 8))])
            p += 8 + 4 * n
        # 指针位置 → 所在补丁组
        self.slot_group = {}
        for g, (_, offsets) in enumerate(self.patches):
            for o in offsets:
                self.slot_group[o] = g
        self.slots = sorted(self.slot_group)

    def append(self, data):
        self.data.extend(b"\0" * (-len(self.data) % ALIGN))
        offset = len(self.data)
        self.data.extend(data)
        return offset

    def add_item(self, flags, offset, count):
        self.items.append([flags, offset, count])
        return len(self.items) - 1

    def slots_in(self, start, size):
        lo = bisect.bisect_left(self.slots, start)
        hi = bisect.bisect_left(self.slots, start + size)
        return self.slots[lo:hi]

    def add_slot(self, position, group):
        self.patches[group][1].append(position)

    def read_ptr(self, position):
        return struct.unpack_from("<I", self.data, position)[0]

    def read_string(self, position):
        item = self.read_ptr(position)
        if not item:
            return ""
        _, offset, count = self.items[item]
        return bytes(self.data[offset:offset + count]).split(b"\0")[0].decode("latin-1")

    def write(self):
        data = bytes(self.data) + b"\0" * (-len(self.data) % POINTER_SIZE)
        items =b"".join(struct.pack("<III", *i) for i in self.items)
        patches = b"".join(struct.pack("<II", ty, len(o)) + struct.pack(f"<{len(o)}I", *o) for ty, o in self.patches)
        head = lambda tag, flags, body: struct.pack(">I", flags | (len(body) + SECTION_HEAD)) + tag.encode("latin-1") + body
        index_parts = []
        for tag, _, _, flags in self.index_secs:
            body = items if tag == "ITEM" else patches if tag == "PTCH" else None
            if body is None:
                raise ValueError(f"unexpected section {tag} in INDX")
            index_parts.append(head(tag, flags, body))
        out = []
        for tag, start, size, flags in self.secs:
            if tag == "DATA":
                out.append(head(tag, flags, data))
            elif tag == "INDX":
                out.append(head(tag, flags, b"".join(index_parts)))
            else:
                out.append(self.buf[start:start + size])
        return head("TAG0", self.root[3], b"".join(out))


def add_clips(tag, wanted):
    types = tag.types
    clip_type = types.find("hkbClipGenerator")
    selector_type = types.find("CustomManualSelectorGenerator")
    clip_fields = types.fields(clip_type)
    selector_fields = types.fields(selector_type)
    clip_size = types.info[clip_type]["size"]
    name_at = clip_fields["name"]
    anim_at = clip_fields["animationName"]
    internal_at = clip_fields["animationInternalId"]
    generators_at = selector_fields["generators"]
    anim_id_at = selector_fields["animId"]

    clips = {}
    internal_ids = {}
    selectors = {}
    for i, (flags, offset, count) in enumerate(tag.items):
        if flags & TYPE_MASK == clip_type:
            anim = tag.read_string(offset + anim_at)
            clips[i] = anim
            internal_ids[anim] = struct.unpack_from("<h", tag.data, offset + internal_at)[0]
    for i, (flags, offset, count) in enumerate(tag.items):
        if flags & TYPE_MASK == selector_type:
            array = tag.read_ptr(offset + generators_at)
            n = tag.items[array][2] if array else 0
            kids = list(struct.unpack_from(f"<{n}Q", tag.data, tag.items[array][1])) if array else []
            anim = struct.unpack_from("<i", tag.data, offset + anim_id_at)[0]
            selectors.setdefault(anim, []).append({"item": i, "offset": offset, "array": array, "kids": kids, "new": []})

    next_internal = max(internal_ids.values()) + 1
    string_items = {}
    string_flags = tag.items[tag.read_ptr(tag.items[next(iter(clips))][1] + name_at)][0]
    report = {"clips": 0, "selectors": set(), "no_selector": [], "exists": 0}
    for anim in wanted:
        section, action = int(anim[1:4]), int(anim[5:])
        same_family = FAMILIES[family_of(section)]
        attached = 0
        for sel in selectors.get(action, []):
            names = [clips.get(k, "") for k in sel["kids"]]
            family = [n for n in names if n and same_family(int(n[1:4]))]
            if not family:
                continue
            if anim in names:
                report["exists"] += 1
                attached += 1
                continue
            template = next(k for k in sel["kids"] if clips.get(k, "") == family[0])
            if anim not in string_items:
                raw = anim.encode("latin-1") + b"\0"
                string_items[anim] = tag.add_item(string_flags, tag.append(raw), len(raw))
                internal_ids[anim] = next_internal
                next_internal += 1
            t_flags, t_offset, _ = tag.items[template]
            body = bytearray(tag.data[t_offset:t_offset + clip_size])
            struct.pack_into("<Q", body, name_at, string_items[anim])
            struct.pack_into("<Q", body, anim_at, string_items[anim])
            struct.pack_into("<h", body, internal_at, internal_ids[anim])
            offset = tag.append(body)
            for slot in tag.slots_in(t_offset, clip_size):
                tag.add_slot(offset + slot - t_offset, tag.slot_group[slot])
            item = tag.add_item(t_flags, offset, 1)
            clips[item] = anim
            sel["kids"].append(item)
            sel["new"].append(item)
            report["clips"] += 1
            attached += 1
        if not attached:
            report["no_selector"].append(anim)

    for groups in selectors.values():
        for sel in groups:
            if not sel["new"]:
                continue
            old = tag.items[sel["array"]]
            group = tag.slot_group[old[1]]
            offset = tag.append(struct.pack(f"<{len(sel['kids'])}Q", *sel["kids"]))
            for k in range(len(sel["kids"])):
                tag.add_slot(offset + k * POINTER_SIZE, group)
            array = tag.add_item(old[0], offset, len(sel["kids"]))
            struct.pack_into("<I", tag.data, sel["offset"] + generators_at, array)
            report["selectors"].add(sel["item"])
    if next_internal > 0x7FFF:
        raise ValueError("animationInternalId overflow")
    return report


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    source, target, listing = sys.argv[1:]
    buf = open(source, "rb").read()
    tag = Tagfile(buf)
    if tag.write() != buf:
        raise ValueError("round trip differs before any change; refusing to write")
    wanted = [l.strip() for l in open(listing, encoding="utf-8") if l.strip()]
    report = add_clips(tag, wanted)
    out = tag.write()
    open(target, "wb").write(out)
    print(f"新增片段 {report['clips']} 个，挂到 {len(report['selectors'])} 个选择器；已存在 {report['exists']}；文件 {len(buf)} -> {len(out)}")
    print(f"没有选择器的动作 {len(report['no_selector'])} 个: {' '.join(report['no_selector'])}")


if __name__ == "__main__":
    main()
