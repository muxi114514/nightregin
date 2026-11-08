"""把样条压缩动画（Havok TAG0，SDK 20180100）里一根骨骼的旋转整体绕局部 Z 轴转 180°，文件大小不变。

阎魔刀把 L_Weapon 的翻转从动作挪到假点上（ga_moveset.ps1 / ga_chr_dummy.ps1），用它抵消动作里那一半。
每个控制点右乘同一个四元数，样条形状和别的骨骼都不动。解析布局同 hkx_spline.ps1。
用法：hkx_flip_bone.py <输入.hkx> <输出.hkx> <骨骼号>
"""
import math
import struct
import sys

SECTION_HEAD = 8
ANIM_TRACKS = 0x20
ANIM_FRAME_INFO = 0x40
ANIM_BLOCK_OFFSETS = 0x60
ANIM_DATA = 0xA0
ROT_SIZE = {0: 4, 1: 5, 2: 6, 3: 3, 4: 2, 5: 16}
ROT_ALIGN = {0: 4, 1: 1, 2: 2, 3: 1, 4: 2, 5: 4}
# 40 位旋转：三个 12 位分量 + 最大分量的位置（2 位）+ 最大分量的符号（1 位）
QUANT_40 = 1
Q40_MASK = (1 << 12) - 1
Q40_ZERO = Q40_MASK >> 1
Q40_STEP = 0.000345436
Q40_TOLERANCE = 0.002
# 转换后每个分量都可能变化，标记写成「全部静态 / 全部样条」
ROT_ALL_STATIC = 0x0F
ROT_ALL_SPLINE = 0xF0
Z_180 = (0.0, 0.0, 1.0, 0.0)


def sections(b, start, end):
    p = start
    while p + SECTION_HEAD <= end:
        size = struct.unpack_from(">I", b, p)[0] & 0x3FFFFFFF
        yield b[p + 4:p + 8].decode("latin-1"), p + SECTION_HEAD, p + size
        p += size


def section(items, tag):
    for s in items:
        if s[0] == tag:
            return s
    raise ValueError(f"section {tag} not found")


def aligned(p, n, base):
    r = (p - base) % n
    return p if r == 0 else p + n - r


def decode_q40(b, p):
    v = int.from_bytes(b[p:p + 5], "little")
    t = [(((v >> (12 * i)) & Q40_MASK) - Q40_ZERO) * Q40_STEP for i in range(3)]
    largest = (v >> 36) & 3
    w = math.sqrt(max(0.0, 1.0 - sum(c * c for c in t)))
    if (v >> 38) & 1:
        w = -w
    return t[:largest] + [w] + t[largest:]


def encode_q40(q):
    largest = max(range(4), key=lambda i: abs(q[i]))
    rest = [q[i] for i in range(4) if i != largest]
    v = 0
    for i, c in enumerate(rest):
        v |= min(Q40_MASK, max(0, round(c / Q40_STEP) + Q40_ZERO)) << (12 * i)
    v |= largest << 36
    if q[largest] < 0:
        v |= 1 << 38
    return v.to_bytes(5, "little")


def mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz]


def skip_vector(b, p, flags, quant, blob):
    splines = 0
    count = 0
    if flags & 0x70:
        count = struct.unpack_from("<H", b, p)[0]
        p = aligned(p + 3 + count + b[p + 2] + 2, 4, blob)
    for c in range(3):
        if flags & (0x10 << c):
            p += 8
            splines += 1
        elif flags & (1 << c):
            p += 4
    if splines:
        p += (count + 1) * splines * (1 if quant == 0 else 2)
    return aligned(p, 4, blob)


def rotation_points(b, p, flags, quant, blob):
    """返回 (控制点位置列表, 轨道结束位置)"""
    size = ROT_SIZE[quant]
    points = []
    if flags & 0xF0:
        count = struct.unpack_from("<H", b, p)[0]
        p = aligned(p + 3 + count + b[p + 2] + 2, ROT_ALIGN[quant], blob)
        points = [p + i * size for i in range(count + 1)]
        p += (count + 1) * size
    elif flags & 0x0F:
        p = aligned(p, ROT_ALIGN[quant], blob)
        points = [p]
        p += size
    return points, aligned(p, 4, blob)


def flip_bone(b, bone):
    tag0 = section(list(sections(b, 0, len(b))), "TAG0")
    inner = list(sections(b, tag0[1], tag0[2]))
    data = section(inner, "DATA")
    index = section(inner, "INDX")
    item = section(list(sections(b, index[1], index[2])), "ITEM")
    items = [struct.unpack_from("<ii", b, at + 4) for at in range(item[1], item[2] - 11, 12)]
    data_size = data[2] - data[1]
    offsets = sorted({o for o, _ in items})
    following = {o: (offsets[i + 1] if i + 1 < len(offsets) else data_size) for i, o in enumerate(offsets)}

    # 动画数据是最大的字节数组，动画对象的数据字段指向它
    blob_index, blob_size = -1, 0
    for i, (o, c) in enumerate(items):
        size = following[o] - o
        if c > 0 and abs(size - c) < 16 and size > blob_size:
            blob_index, blob_size = i, size
    anim = None
    for o, c in items:
        if c == 1 and o + ANIM_DATA + 8 <= data_size and struct.unpack_from("<i", b, data[1] + o + ANIM_DATA)[0] == blob_index:
            anim = data[1] + o
    if anim is None:
        raise ValueError("spline animation not found")
    tracks = struct.unpack_from("<i", b, anim + ANIM_TRACKS)[0]
    blocks = struct.unpack_from("<i", b, anim + ANIM_FRAME_INFO + 4)[0]
    mask_size = struct.unpack_from("<i", b, anim + ANIM_FRAME_INFO + 12)[0]
    block_item = items[struct.unpack_from("<i", b, anim + ANIM_BLOCK_OFFSETS)[0]]

    # 轨道 → 骨骼：长度等于轨道数、元素互不相同的 s16 数组
    track = -1
    for o, c in items:
        if c != tracks or (following[o] - o) // tracks != 2:
            continue
        mapping = struct.unpack_from(f"<{tracks}h", b, data[1] + o)
        if len(set(mapping)) == tracks and bone in mapping:
            track = mapping.index(bone)
            break
    if track < 0:
        raise ValueError(f"bone {bone} has no track")

    blob = data[1] + items[blob_index][0]
    flipped = 0
    for block in range(blocks):
        block_offset = struct.unpack_from("<i", b, data[1] + block_item[0] + block * 4)[0]
        masks = blob + block_offset
        cursor = aligned(masks + mask_size, 4, blob)
        for t in range(tracks):
            quant, pos_flags, rot_flags, scale_flags = b[masks + t * 4:masks + t * 4 + 4]
            rot_quant = (quant >> 2) & 15
            cursor = skip_vector(b, cursor, pos_flags, quant & 3, blob)
            points, cursor = rotation_points(b, cursor, rot_flags, rot_quant, blob)
            cursor = skip_vector(b, cursor, scale_flags, (quant >> 6) & 3, blob)
            if t != track:
                continue
            if not points:
                raise ValueError(f"block {block}: bone {bone} has no stored rotation, cannot flip in place")
            if rot_quant != QUANT_40:
                raise ValueError(f"block {block}: rotation quantization {rot_quant} not supported")
            for p in points:
                target = mul(decode_q40(b, p), Z_180)
                b[p:p + 5] = encode_q40(target)
                if max(abs(x - y) for x, y in zip(decode_q40(b, p), target)) > Q40_TOLERANCE:
                    raise ValueError(f"block {block}: quaternion round trip failed")
            b[masks + t * 4 + 2] = ROT_ALL_SPLINE if len(points) > 1 or rot_flags & 0xF0 else ROT_ALL_STATIC
            flipped += len(points)
    return flipped


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    source, target, bone = sys.argv[1:]
    buf = bytearray(open(source, "rb").read())
    flipped = flip_bone(buf, int(bone))
    open(target, "wb").write(buf)
    print(f"翻转 {flipped} 个控制点")


if __name__ == "__main__":
    main()
