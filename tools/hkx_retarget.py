"""按骨骼名把别的角色的样条压缩动画（Havok TAG0，SDK 20180100）改绑到另一套骨骼。

熔炉骑士 c2500 的动作搬到玩家 c0000 用（两边动画引用同一份 compendium，文件能直接放进玩家的分包）：
  对照表：轨道 → 骨骼按骨骼名改成目标骨骼号，目标没有的骨骼（翅膀、喉囊、尾巴等）的轨道删掉（hkx_tracks.py；
          填 -1 引擎加载时会写坏堆，启动随机闪退）；
  旋转：两套骨骼的静止朝向不同（玩家的 Spine 挂在 RootRotY / RootRotXZ 下，骑士的 Root 与玩家 RootPos 差 180°），
        新 = L × 原 × R：R = 来源静止世界朝向⁻¹ × 目标的（每根骨骼相对静止姿势在世界里转过的角度与来源一致），
        L 抵消父链上的校正与目标多出来的静止骨骼；结果对齐到目标静止旋转的半球（与别的动作混合时不绕远路）；
        来源没存、换算后又不为单位的旋转（骑士骨盆）与没存的位移分量（骑士 Spine）补进轨道（hkx_tracks.inject）；
        常数四元数相乘不改变样条插值，控制点逐个换算；
  位移：目标参考位移 + 身高比 × L(原值 − 来源参考位移)，静态值直接改、样条改量化上下界（L 只差正负号时）；
  根运动（参考帧采样）按身高比缩放，原骨架名换成目标的
用法：hkx_retarget.py <来源动画.hkx> <来源骨骼.hkx> <目标骨骼.hkx> <输出.hkx> [来源骨骼名=目标骨骼名 ...]
"""
import math
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hkx_flip_bone import (ANIM_BLOCK_OFFSETS, ANIM_DATA, ANIM_FRAME_INFO, ANIM_TRACKS, QUANT_40, ROT_ALL_SPLINE, ROT_ALL_STATIC,
                           aligned, decode_q40, encode_q40, mul, rotation_points, section, sections, skip_vector)
from hkx_tracks import drop_tracks

# 名字不同但是同一根骨骼
DEFAULT_ALIASES = {"Root": "RootPos"}
# 身高比取大腿 + 小腿长度（L_Calf、L_Foot 相对父骨骼的位移）
LEG_BONES = ("L_Calf", "L_Foot")
SKELETON_NAME = re.compile(rb"^c\d{4}\x00$")
# 来源没存（旋转默认单位、位移默认 0）、换算后又差出这么多的分量补进轨道（hkx_tracks.inject）
INJECT_TOLERANCE = 1e-4
# 补偿小于这个角度（度）的骨骼不改旋转，免得重新量化引入误差
ANGLE_EPSILON = 0.05
IDENTITY = (0.0, 0.0, 0.0, 1.0)
# 直接对齐世界朝向的目标骨骼（武器挂点，见 frame_fix）
WORLD_MATCH = {"R_Weapon", "L_Weapon"}
SIGN_TOLERANCE = 1e-3


class Tagfile:
    def __init__(self, b):
        tag0 = section(list(sections(b, 0, len(b))), "TAG0")
        inner = list(sections(b, tag0[1], tag0[2]))
        data = section(inner, "DATA")
        index = section(inner, "INDX")
        item = section(list(sections(b, index[1], index[2])), "ITEM")
        self.b = b
        self.base = data[1]
        self.items = [struct.unpack_from("<ii", b, at + 4) for at in range(item[1], item[2] - 11, 12)]
        size = data[2] - data[1]
        offsets = sorted({o for o, _ in self.items})
        self.following = {o: (offsets[i + 1] if i + 1 < len(offsets) else size) for i, o in enumerate(offsets)}

    def arrays(self, element):
        """元素大小为 element 的数组条目：[(编号, 偏移, 数量)]"""
        out = []
        for i, (o, c) in enumerate(self.items):
            if c > 0 and 0 <= self.following[o] - o - element * c < 16:
                out.append((i, o, c))
        return out

    def string(self, index):
        o, c = self.items[index]
        return bytes(self.b[self.base + o:self.base + o + c])


def inverse(q):
    return (-q[0], -q[1], -q[2], q[3])


def angle(q):
    return math.degrees(2 * math.acos(min(1.0, abs(q[3]))))


def matrix(q):
    x, y, z, w = q
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]


class Skeleton:
    def __init__(self, path):
        t = Tagfile(open(path, "rb").read())
        self.names = None
        # 骨骼数组：每个 16 字节，开头是名字字符串的条目编号；布料骨骼名带 [cloth] 前缀
        for _, o, c in t.arrays(16):
            names = []
            for i in range(c):
                ref = struct.unpack_from("<I", t.b, t.base + o + i * 16)[0]
                raw = t.string(ref) if 0 < ref < len(t.items) else b""
                if not re.match(rb"^[\x21-\x7e][\x20-\x7e]*\x00$", raw):
                    break
                names.append(raw[:-1].decode("ascii"))
            if len(names) == c and c > 10:
                self.names = names
                break
        if not self.names:
            raise ValueError(f"{path}: bone array not found")
        n = len(self.names)
        pose = [o for _, o, c in t.arrays(48) if c == n]
        if len(pose) != 1:
            raise ValueError(f"{path}: reference pose not found")
        self.translations = [struct.unpack_from("<3f", t.b, t.base + pose[0] + i * 48) for i in range(n)]
        self.rotations = rotations = [struct.unpack_from("<4f", t.b, t.base + pose[0] + i * 48 + 16) for i in range(n)]
        self.parents = None
        for _, o, c in t.arrays(2):
            if c == n:
                v = struct.unpack_from(f"<{n}h", t.b, t.base + o)
                if v[0] == -1 and all(-1 <= p < i for i, p in enumerate(v)):
                    self.parents = v
        if self.parents is None:
            raise ValueError(f"{path}: parent indices not found")
        self.world = []
        for i, p in enumerate(self.parents):
            self.world.append(rotations[i] if p < 0 else mul(self.world[p], rotations[i]))
        self.name = next((t.string(i) for i in range(1, len(t.items)) if SKELETON_NAME.match(t.string(i))), None)

    def leg_length(self):
        return sum(math.sqrt(sum(v * v for v in self.translations[self.names.index(n)])) for n in LEG_BONES)


def vector_layout(b, p, flags, quant, blob):
    """同 skip_vector 的走法，返回 ([(种类, 位置)] × 3, 结束位置)；种类 spline 位置指向上下界，static 指向值，None 为没存"""
    count = 0
    if flags & 0x70:
        count = struct.unpack_from("<H", b, p)[0]
        p = aligned(p + 3 + count + b[p + 2] + 2, 4, blob)
    parts = []
    splines = 0
    for c in range(3):
        if flags & (0x10 << c):
            parts.append(("spline", p))
            p += 8
            splines += 1
        elif flags & (1 << c):
            parts.append(("static", p))
            p += 4
        else:
            parts.append((None, p))
    if splines:
        p += (count + 1) * splines * (1 if quant == 0 else 2)
    return parts, aligned(p, 4, blob)


def frame_fix(src, dst, track):
    """目标骨骼世界朝向 = 来源的 × 这个常数（来源静止世界朝向⁻¹ × 目标的），即相对静止姿势转过的角度与来源一致；
    武器骨骼的模型都沿局部 +Y（刀身朝刀尖）挂，两边静止朝向不同也该让刀身世界朝向一致，常数取单位"""
    if dst.names[track[1]] in WORLD_MATCH:
        return IDENTITY
    return mul(inverse(src.world[track[0]]), dst.world[track[1]])


class TrackPlan:
    def __init__(self, src, dst, ratio, track, matched):
        """matched：目标骨骼号 → 来源骨骼号。目标父链上没有轨道的骨骼保持静止局部，跟着最近的有轨道祖先 a 走：
        目标父骨骼世界 = 来源(a) × 校正(a) × K，K 为 a 到父骨骼之间的静止局部之积，所以 L = (校正(a) × K)⁻¹"""
        self.track = track
        a = dst.parents[track[1]]
        while a >= 0 and a not in matched:
            a = dst.parents[a]
        parent = dst.parents[track[1]]
        parent_rest = dst.world[parent] if parent >= 0 else IDENTITY
        if a >= 0:
            k = mul(inverse(dst.world[a]), parent_rest)
            fix = frame_fix(src, dst, (matched[a], a))
        else:
            k, fix = parent_rest, IDENTITY
        # 来源的父骨骼要正好是 a 对应的那根，否则局部旋转之间还夹着别的动画骨骼，常数补偿不成立
        self.hierarchy_ok = src.parents[track[0]] == (matched[a] if a >= 0 else -1)
        self.left = inverse(mul(fix, k))
        self.right = frame_fix(src, dst, track)
        self.rotate = angle(self.left) > ANGLE_EPSILON or angle(self.right) > ANGLE_EPSILON
        self.m = matrix(self.left)
        self.ratio = ratio
        self.src_ref = src.translations[track[0]]
        self.dst_ref = dst.translations[track[1]]
        self.rest = dst.rotations[track[1]]

    def translation(self, v):
        d = [v[c] - self.src_ref[c] for c in range(3)]
        return [self.dst_ref[r] + self.ratio * sum(self.m[r][c] * d[c] for c in range(3)) for r in range(3)]

    def diagonal(self):
        """L 只差正负号时每个分量单独映射；否则返回 None"""
        signs = []
        for r in range(3):
            if any(abs(self.m[r][c]) > SIGN_TOLERANCE for c in range(3) if c != r) or abs(abs(self.m[r][r]) - 1) > SIGN_TOLERANCE:
                return None
            signs.append(1.0 if self.m[r][r] > 0 else -1.0)
        return signs

    def same_side(self, q):
        """四元数取与目标静止旋转同一半球，混合时不会绕远路"""
        return q if sum(a * b for a, b in zip(q, self.rest)) >= 0 else [-x for x in q]


def edit_translation(b, plan, parts, note):
    """改存着的位移；返回没存、换算后又不为 0 的分量 {分量: 值}（由 hkx_tracks.inject 补进去）"""
    kinds = [k for k, _ in parts]
    values = [struct.unpack_from("<f", b, p)[0] if k == "static" else 0.0 for k, p in parts]
    missing = {}
    if "spline" not in kinds:
        # 全是静态（或没存）：整个向量换算
        target = plan.translation(values)
        for c, (k, p) in enumerate(parts):
            if k == "static":
                struct.pack_into("<f", b, p, target[c])
            elif abs(target[c]) > INJECT_TOLERANCE:
                missing[c] = target[c]
        return missing
    signs = plan.diagonal()
    if signs is None:
        note(plan, "spline translation under a non axis-aligned parent fix", 1.0)
        signs = [1.0 if plan.m[c][c] >= 0 else -1.0 for c in range(3)]
    for c, (k, p) in enumerate(parts):
        scale = plan.ratio * signs[c]
        offset = plan.dst_ref[c] - scale * plan.src_ref[c]
        if k == "spline":
            lo, hi = struct.unpack_from("<2f", b, p)
            struct.pack_into("<2f", b, p, lo * scale + offset, hi * scale + offset)
        elif k == "static":
            struct.pack_into("<f", b, p, values[c] * scale + offset)
        elif abs(offset) > INJECT_TOLERANCE:
            missing[c] = offset
    return missing


def edit_rotation(b, plan, mask, rot_flags, rot_quant, points):
    """存着的旋转逐个控制点换算并对齐半球；没存的返回要补的静态旋转。返回 (是否改了, 要补的旋转或 None)"""
    if not points:
        fixed = mul(plan.left, plan.right)
        return False, (plan.same_side(fixed) if angle(fixed) > ANGLE_EPSILON else None)
    if rot_quant != QUANT_40:
        if plan.rotate:
            raise ValueError(f"{plan.track}: rotation quantization {rot_quant} not supported")
        return False, None
    qs = [decode_q40(b, p) for p in points]
    if plan.rotate:
        qs = [mul(mul(plan.left, q), plan.right) for q in qs]
    flip = sum(a * c for a, c in zip(qs[0], plan.rest)) < 0
    if not plan.rotate and not flip:
        return False, None
    for p, q in zip(points, qs):
        b[p:p + 5] = encode_q40([-x for x in q] if flip else q)
    b[mask + 2] = ROT_ALL_SPLINE if rot_flags & 0xF0 else ROT_ALL_STATIC
    return True, None


def retarget(b, src, dst, aliases):
    t = Tagfile(b)
    blob_index, blob_size = -1, 0
    for i, (o, c) in enumerate(t.items):
        size = t.following[o] - o
        if c > 0 and abs(size - c) < 16 and size > blob_size:
            blob_index, blob_size = i, size
    anim = next((t.base + o for o, c in t.items if c == 1 and o + ANIM_DATA + 8 <= len(b)
                 and struct.unpack_from("<i", b, t.base + o + ANIM_DATA)[0] == blob_index), None)
    if anim is None:
        raise ValueError("spline animation not found")
    tracks = struct.unpack_from("<i", b, anim + ANIM_TRACKS)[0]
    frames, blocks, _, mask_size = struct.unpack_from("<4i", b, anim + ANIM_FRAME_INFO)
    block_item = t.items[struct.unpack_from("<i", b, anim + ANIM_BLOCK_OFFSETS)[0]]
    binding = [(i, o) for i, o, c in t.arrays(2) if c == tracks and len(set(struct.unpack_from(f"<{c}h", b, t.base + o))) == c]
    if len(binding) != 1:
        raise ValueError(f"track-to-bone table not found ({len(binding)} candidates)")
    binding_item, table = binding[0][0], t.base + binding[0][1]
    ratio = dst.leg_length() / src.leg_length()

    pairs, unbound = [], []
    for s in struct.unpack_from(f"<{tracks}h", b, table):
        name = aliases.get(src.names[s], src.names[s])
        if name in dst.names:
            pairs.append((s, dst.names.index(name)))
        else:
            pairs.append(None)
            unbound.append(src.names[s])
    matched = {d: s for s, d in (p for p in pairs if p)}
    plans = [TrackPlan(src, dst, ratio, p, matched) if p else None for p in pairs]
    bound = [p.track[1] if p else -1 for p in plans]
    struct.pack_into(f"<{tracks}h", b, table, *bound)

    blob = t.base + t.items[blob_index][0]
    issues, injections = {}, {}
    rotated = 0

    def note(plan, what, size):
        key = f"{dst.names[plan.track[1]]}:{what}"
        issues[key] = max(issues.get(key, 0), size)

    for block in range(blocks):
        block_offset = struct.unpack_from("<i", b, t.base + block_item[0] + block * 4)[0]
        masks = blob + block_offset
        cursor = aligned(masks + mask_size, 4, blob)
        for track in range(tracks):
            mask = masks + track * 4
            quant, pos_flags, rot_flags, scale_flags = b[mask:mask + 4]
            plan = plans[track]
            parts, cursor = vector_layout(b, cursor, pos_flags, quant & 3, blob)
            rot_quant = (quant >> 2) & 15
            missing = edit_translation(b, plan, parts, note) if plan else {}
            points, cursor = rotation_points(b, cursor, rot_flags, rot_quant, blob)
            if plan:
                if not plan.hierarchy_ok and block == 0:
                    note(plan, "different parent", 1.0)
                changed, rotation = edit_rotation(b, plan, mask, rot_flags, rot_quant, points)
                rotated += changed
                if rotation is not None or missing:
                    injections[track] = (rotation, missing)
            cursor = skip_vector(b, cursor, scale_flags, (quant >> 6) & 3, blob)
        limit = blob + (struct.unpack_from("<i", b, t.base + block_item[0] + (block + 1) * 4)[0] if block + 1 < blocks else blob_size)
        if cursor > limit:
            raise ValueError(f"block {block}: walked past the block end ({cursor - limit} bytes)")

    # 根运动：参考帧采样（hkVector4，xyz 位移 + w 朝向），数量与帧数相近
    roots = [(o, c) for _, o, c in t.arrays(16) if abs(c - frames) <= 8 and c > 1]
    for o, c in roots:
        for i in range(c):
            at = t.base + o + i * 16
            x, y, z = struct.unpack_from("<3f", b, at)
            struct.pack_into("<3f", b, at, x * ratio, y * ratio, z * ratio)
    renamed = 0
    for o, c in t.items:
        if c == len(dst.name) and SKELETON_NAME.match(bytes(b[t.base + o:t.base + o + c])):
            b[t.base + o:t.base + o + c] = dst.name
            renamed += 1
    return {"tracks": tracks, "frames": frames, "blocks": blocks, "ratio": ratio, "unbound": unbound, "rotated": rotated,
            "issues": issues, "root_samples": sum(c for _, c in roots), "renamed": renamed, "injections": injections,
            "binding_item": binding_item, "keep": [i for i, bone in enumerate(bound) if bone >= 0], "plans": plans}


def check(b, dst):
    """读回：对照表没有负数和重复、骨骼号在范围内，每块走完不越界"""
    t = Tagfile(b)
    blob_index = max(range(len(t.items)), key=lambda i: t.items[i][1] if t.items[i][1] > 0 and abs(t.following[t.items[i][0]] - t.items[i][0] - t.items[i][1]) < 16 else 0)
    anim = next(t.base + o for o, c in t.items if c == 1 and struct.unpack_from("<i", b, t.base + o + ANIM_DATA)[0] == blob_index)
    tracks = struct.unpack_from("<i", b, anim + ANIM_TRACKS)[0]
    _, blocks, _, mask_size = struct.unpack_from("<4i", b, anim + ANIM_FRAME_INFO)
    tables = [struct.unpack_from(f"<{c}h", b, t.base + o) for _, o, c in t.arrays(2) if c == tracks]
    if not any(min(v) >= 0 and max(v) < len(dst.names) and len(set(v)) == len(v) for v in tables):
        raise ValueError("self-check: no valid track-to-bone table")
    blob = t.base + t.items[blob_index][0]
    block_item = t.items[struct.unpack_from("<i", b, anim + ANIM_BLOCK_OFFSETS)[0]]
    for k in range(blocks):
        start = struct.unpack_from("<i", b, t.base + block_item[0] + k * 4)[0]
        cursor = aligned(blob + start + mask_size, 4, blob)
        for tr in range(tracks):
            quant, pf, rf, sf = b[blob + start + tr * 4:blob + start + tr * 4 + 4]
            cursor = skip_vector(b, cursor, pf, quant & 3, blob)
            _, cursor = rotation_points(b, cursor, rf, (quant >> 2) & 15, blob)
            cursor = skip_vector(b, cursor, sf, (quant >> 6) & 3, blob)
        if cursor - blob > t.items[blob_index][1]:
            raise ValueError(f"self-check: block {k} runs past the data")
    return tracks


def main():
    if len(sys.argv) < 5:
        raise SystemExit(__doc__)
    src_anim, src_skel, dst_skel, out = sys.argv[1:5]
    aliases = dict(DEFAULT_ALIASES)
    aliases.update(dict(a.split("=", 1) for a in sys.argv[5:]))
    src, dst = Skeleton(src_skel), Skeleton(dst_skel)
    b = bytearray(open(src_anim, "rb").read())
    r = retarget(b, src, dst, aliases)
    # 对照表里的 -1 引擎不认（加载时写坏堆），没对上的轨道真删掉；没存的旋转 / 位移补进去
    result = drop_tracks(b, r["binding_item"], r["keep"], r["injections"])
    kept = check(result, dst)
    open(out, "wb").write(result)
    injected = " ".join(dst.names[r["plans"][t].track[1]] + ("(rot)" if rot is not None else "") + ("(pos " + "".join("xyz"[c] for c in tr) + ")" if tr else "")
                        for t, (rot, tr) in sorted(r["injections"].items()))
    print(f"tracks {r['tracks']} -> {kept} ({r['frames']} frames, {r['blocks']} blocks), ratio {r['ratio']:.3f}, rotated {r['rotated']}")
    print(f"dropped {r['tracks'] - kept}: {' '.join(r['unbound'])}")
    print(f"injected: {injected}")
    print(f"root samples {r['root_samples']}, renamed {r['renamed']}")
    if r["issues"]:
        print("issues: " + " ".join(f"{k}({v:.3f})" for k, v in sorted(r["issues"].items())))


if __name__ == "__main__":
    main()
