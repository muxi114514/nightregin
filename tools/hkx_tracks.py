"""从样条压缩动画（Havok TAG0，SDK 20180100）里删掉轨道、给轨道补上没存的数据，重排文件。

原版动画的轨道 → 骨骼对照表从不出现负数或重复骨骼；对照表填 -1 会让引擎加载时写坏堆（启动随机闪退），
所以别的角色的动画里目标骨骼没有的轨道必须真删：
  每块：掩码与轨道数据都只留下要的（每条轨道的数据从 4 字节对齐处开始，原样搬运，内部对齐不变）；
  动画对象：变换轨道数（+0x20）、掩码大小（+0x4C）；块偏移 / 浮点块偏移按新块重算；
  数据块之后的条目（绑定对象、对照表、骨架名、参考帧等）按原对齐重新排位，条目表与指针补丁表（PTCH）的偏移一起改。
TAG0 的数组长度只记在条目表里，对象里的长度字段为 0，不用改
"""
import struct

from hkx_flip_bone import (ANIM_BLOCK_OFFSETS, ANIM_DATA, ANIM_FRAME_INFO, ANIM_TRACKS, QUANT_40, aligned, encode_q40, rotation_points, section,
                           sections, skip_vector)

ANIM_FLOAT_TRACKS = 0x24
ANIM_FLOAT_BLOCK_OFFSETS = 0x70
BLOCK_ALIGN = 16
SIZE_MASK = 0x3FFFFFFF


def _header(flags_word, size):
    return struct.pack(">I", (flags_word & ~SIZE_MASK & 0xFFFFFFFF) | size)


def _alignment(offset):
    for a in (16, 8, 4, 2):
        if offset % a == 0:
            return a
    return 1


def track_chunks(b, blob, block_start, tracks, mask_size):
    """一块里每条轨道数据的 [起, 止)（相对数据块开头），以及变换数据结束位置"""
    masks = blob + block_start
    cursor = aligned(masks + mask_size, 4, blob)
    chunks = []
    for t in range(tracks):
        quant, pos_flags, rot_flags, scale_flags = b[masks + t * 4:masks + t * 4 + 4]
        start = cursor
        cursor = skip_vector(b, cursor, pos_flags, quant & 3, blob)
        _, cursor = rotation_points(b, cursor, rot_flags, (quant >> 2) & 15, blob)
        cursor = skip_vector(b, cursor, scale_flags, (quant >> 6) & 3, blob)
        chunks.append((start - blob, cursor - blob))
    return chunks, cursor - blob


def inject(mask, chunk, rotation=None, translation=None):
    """给一条轨道补上没存的数据：rotation = 没存旋转时写成静态 40 位四元数；translation = {分量: 值} 补没存的位移分量。
    chunk 从 4 字节对齐处开始，按相对开头 0 走（轨道内最多 4 字节对齐，相对位置不变）。返回 (新掩码, 新数据)"""
    mask = bytearray(mask)
    quant, pos_flags, rot_flags, scale_flags = mask
    buf = bytes(chunk)
    p = 0
    if pos_flags & 0x70:
        count = struct.unpack_from("<H", buf, 0)[0]
        p = aligned(3 + count + buf[2] + 2, 4, 0)
    head = buf[:p]
    values = []
    splines = 0
    for c in range(3):
        if pos_flags & (0x10 << c):
            values.append(buf[p:p + 8]); p += 8; splines += 1
        elif pos_flags & (1 << c):
            values.append(buf[p:p + 4]); p += 4
        elif translation and c in translation:
            values.append(struct.pack("<f", translation[c])); pos_flags |= 1 << c
        else:
            values.append(b"")
    points = (struct.unpack_from("<H", buf, 0)[0] + 1) * splines * (1 if quant & 3 == 0 else 2) if splines else 0
    rest = buf[p:p + points]
    pos_end = aligned(p + points, 4, 0)
    pos = bytearray(head + b"".join(values) + rest)
    while len(pos) % 4:
        pos.append(0)
    _, rot_end = rotation_points(buf, pos_end, rot_flags, (quant >> 2) & 15, 0)
    rot = buf[pos_end:rot_end]
    if rotation is not None and not rot_flags:
        # 原来没存旋转，压缩格式字段无意义，改成 40 位（静态 5 字节，对齐 1，补到 4 字节）
        mask[0] = (quant & ~(0xF << 2)) | (QUANT_40 << 2)
        rot = encode_q40(rotation) + b"\x00" * 3
        rot_flags = 0x0F
    mask[1], mask[2] = pos_flags, rot_flags
    return bytes(mask), bytes(pos) + bytes(rot) + buf[rot_end:]


def drop_tracks(b, binding_item, keep, injections=None):
    """keep：留下的轨道号（升序）；binding_item：对照表的条目号；injections = {轨道: (旋转, {分量: 位移})} 补没存的数据。返回新文件字节"""
    injections = injections or {}
    tag0 = section(list(sections(b, 0, len(b))), "TAG0")
    inner = list(sections(b, tag0[1], tag0[2]))
    data = section(inner, "DATA")
    index = section(inner, "INDX")
    subs = list(sections(b, index[1], index[2]))
    item_sec, patch_sec = section(subs, "ITEM"), section(subs, "PTCH")
    base = data[1]
    items = [list(struct.unpack_from("<Iii", b, at)) for at in range(item_sec[1], item_sec[2] - 11, 12)]
    size_of = {}
    offsets = sorted({o for _, o, _ in items})
    for i, o in enumerate(offsets):
        size_of[o] = (offsets[i + 1] if i + 1 < len(offsets) else data[2] - data[1]) - o

    blob_index = max(range(len(items)), key=lambda i: items[i][2] if items[i][2] > 0 and abs(size_of[items[i][1]] - items[i][2]) < 16 else 0)
    anim = next(base + o for _, o, c in items if c == 1 and struct.unpack_from("<i", b, base + o + ANIM_DATA)[0] == blob_index)
    tracks = struct.unpack_from("<i", b, anim + ANIM_TRACKS)[0]
    if struct.unpack_from("<i", b, anim + ANIM_FLOAT_TRACKS)[0]:
        raise ValueError("float tracks are not supported")
    frames, blocks, _, mask_size = struct.unpack_from("<4i", b, anim + ANIM_FRAME_INFO)
    if mask_size != tracks * 4:
        raise ValueError(f"mask size {mask_size} != 4 x {tracks} tracks")
    block_item = items[struct.unpack_from("<i", b, anim + ANIM_BLOCK_OFFSETS)[0]]
    float_item = items[struct.unpack_from("<i", b, anim + ANIM_FLOAT_BLOCK_OFFSETS)[0]]
    blob_off = items[blob_index][1]
    blob = base + blob_off
    if items[binding_item][2] != tracks or block_item[2] != blocks or float_item[2] != blocks:
        raise ValueError("unexpected array lengths")

    # 新数据块
    new_blob = bytearray()
    block_offsets, float_offsets = [], []
    for k in range(blocks):
        start = struct.unpack_from("<i", b, base + block_item[1] + k * 4)[0]
        chunks, end = track_chunks(b, blob, start, tracks, mask_size)
        if end - start != struct.unpack_from("<i", b, base + float_item[1] + k * 4)[0]:
            raise ValueError(f"block {k}: transform data end mismatch")
        while len(new_blob) % BLOCK_ALIGN:
            new_blob.append(0)
        block_offsets.append(len(new_blob))
        masks, datas = [], []
        for t in keep:
            mask = bytes(b[blob + start + t * 4:blob + start + t * 4 + 4])
            s, e = chunks[t]
            data = bytes(b[blob + s:blob + e])
            if t in injections:
                mask, data = inject(mask, data, *injections[t])
            masks.append(mask)
            datas.append(data)
        block = bytearray(b"".join(masks))
        while len(block) % 4:
            block.append(0)
        for data in datas:
            block += data
        float_offsets.append(len(block))
        new_blob += block
    while len(new_blob) % BLOCK_ALIGN:
        new_blob.append(0)

    old_binding = struct.unpack_from(f"<{tracks}h", b, base + items[binding_item][1])
    new_binding = struct.pack(f"<{len(keep)}h", *[old_binding[t] for t in keep])

    # 重排 DATA：数据块之前原样（改字段），之后的条目按原对齐依次排
    head = bytearray(b[base:blob])
    struct.pack_into("<i", head, anim - base + ANIM_TRACKS, len(keep))
    struct.pack_into("<i", head, anim - base + ANIM_FRAME_INFO + 12, len(keep) * 4)
    for item, values in ((block_item, block_offsets), (float_item, float_offsets)):
        if item[1] >= blob_off:
            raise ValueError("offset arrays after the data block are not supported")
        struct.pack_into(f"<{blocks}i", head, item[1], *values)
    out = head + new_blob
    moved = {}
    for o in sorted(o for o in offsets if o > blob_off):
        content = new_binding if o == items[binding_item][1] else bytes(b[base + o:base + o + size_of[o]])
        while len(out) % _alignment(o):
            out.append(0)
        moved[o] = (len(out), size_of[o])
        out += content
    while len(out) % 16:
        out.append(0)

    def remap(x):
        if x < blob_off:
            return x
        for o, (n, size) in moved.items():
            if o <= x < o + size:
                return n + (x - o)
        raise ValueError(f"pointer at {x} is inside the data block")

    for it in items:
        if it[1] in moved:
            it[1] = moved[it[1]][0]
    items[blob_index][2] = len(new_blob)
    items[binding_item][2] = len(keep)
    patch = bytearray(b[patch_sec[1]:patch_sec[2]])
    at = 0
    while at < len(patch):
        _, n = struct.unpack_from("<II", patch, at)
        for i in range(n):
            p = at + 8 + i * 4
            struct.pack_into("<I", patch, p, remap(struct.unpack_from("<I", patch, p)[0]))
        at += 8 + 4 * n

    def sec(tag, flags_at, body):
        return _header(struct.unpack_from(">I", b, flags_at)[0], 8 + len(body)) + tag.encode() + bytes(body)

    item_body = b"".join(struct.pack("<Iii", *it) for it in items)
    index_body = sec("ITEM", item_sec[1] - 8, item_body) + sec("PTCH", patch_sec[1] - 8, patch)
    body = bytearray()
    for tag, start, end in inner:
        if tag == "DATA":
            body += sec("DATA", start - 8, out)
        elif tag == "INDX":
            body += sec("INDX", start - 8, index_body)
        else:
            body += b[start - 8:end]
    return bytes(sec("TAG0", tag0[1] - 8, body))
