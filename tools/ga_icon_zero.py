"""由多才多艺 1 层图标做 0 层图标：只换左上角的数字，其余块原样保留。
输入输出都是 40x40 的 BC7 块（100 块、每块 16 字节，无文件头）。图集是 BC7，Pillow 能解不能压，改动的块用模式 6 自己压。
用法：ga_icon_zero.py <输入.bin> <输出.bin> [预览.png]。由 tools\\ga_versatility.ps1 调用，要 Pillow + numpy
"""
import io
import struct
import sys

import numpy as np
from PIL import Image, ImageDraw

ICON = 40
BLOCK = 4
BLOCK_BYTES = 16
SUPERSAMPLE = 8
# 数字占的范围（量过 1~9 层的亮像素：x 7~16、y 4~18），圆环再略放大一点才盖得住原来的「1」
DIGIT_BOX = (6.5, 4, 16.5, 18.5)
RING_WIDTH = 2.6
OUTLINE = 1.0
DIGIT_COLOR = (255, 255, 255, 255)
OUTLINE_COLOR = (20, 18, 14, 255)
# 空心处压暗，盖住原来的数字
COUNTER_COLOR = (38, 36, 28, 255)
DXGI_BC7_UNORM = 98
# 模式 6 的 4 位索引权重
WEIGHTS = np.array([0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64], dtype=np.float64) / 64.0
MODE_6 = 1 << 6
TRANSPARENT = 16
# 按不透明度折算后的最大允许误差。一组端点表示不了「白 + 深色 + 透明」同块，少数边缘像素会偏
MAX_ERROR = 128


def dds(blocks):
    head = struct.pack("<4s7I", b"DDS ", 124, 0x81007, ICON, ICON, len(blocks), 0, 1) + b"\0" * 44
    head += struct.pack("<2I4s5I", 32, 4, b"DX10", 0, 0, 0, 0, 0)
    head += struct.pack("<5I", 0x1000, 0, 0, 0, 0)
    head += struct.pack("<5I", DXGI_BC7_UNORM, 3, 0, 1, 0)
    return head + blocks


def draw_zero(base):
    size = ICON * SUPERSAMPLE
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    pen = ImageDraw.Draw(layer)
    x0, y0, x1, y1 = [v * SUPERSAMPLE for v in DIGIT_BOX]
    ring = RING_WIDTH * SUPERSAMPLE
    edge = OUTLINE * SUPERSAMPLE
    pen.ellipse((x0 - edge, y0 - edge, x1 + edge, y1 + edge), fill=OUTLINE_COLOR)
    pen.ellipse((x0, y0, x1, y1), fill=DIGIT_COLOR)
    pen.ellipse((x0 + ring, y0 + ring, x1 - ring, y1 - ring), fill=OUTLINE_COLOR)
    pen.ellipse((x0 + ring + edge, y0 + ring + edge, x1 - ring - edge, y1 - ring - edge), fill=COUNTER_COLOR)
    layer = layer.resize((ICON, ICON), Image.LANCZOS)
    out = base.copy()
    out.alpha_composite(layer)
    return out


def encode_block(pixels):
    """模式 6：一组端点（RGBA 各 7 位 + 共用 1 位）+ 16 个 4 位索引。
    块里的像素两两试作端点；透明像素的颜色看不见，误差按不透明度加权，作端点时借另一端的颜色"""
    p = pixels.reshape(-1, 4).astype(np.float64)
    weight = np.ones((16, 4))
    weight[:, :3] = (p[:, 3:] / 255.0) ** 2
    best = None
    candidates = np.unique(p, axis=0)
    for i in range(len(candidates)):
        for j in range(i, len(candidates)):
            e0, e1 = candidates[i].copy(), candidates[j].copy()
            if e0[3] < TRANSPARENT:
                e0[:3] = e1[:3]
            if e1[3] < TRANSPARENT:
                e1[:3] = e0[:3]
            for p0 in (0, 1):
                for p1 in (0, 1):
                    a = np.clip(np.round((e0 - p0) / 2), 0, 127).astype(int)
                    b = np.clip(np.round((e1 - p1) / 2), 0, 127).astype(int)
                    ea = (a * 2 + p0).astype(np.float64)
                    eb = (b * 2 + p1).astype(np.float64)
                    palette = np.outer(1 - WEIGHTS, ea) + np.outer(WEIGHTS, eb)
                    dist = (((p[:, None, :] - palette[None, :, :]) ** 2) * weight[:, None, :]).sum(axis=2)
                    index = dist.argmin(axis=1)
                    error = dist.min(axis=1).sum()
                    if best is None or error < best[0]:
                        best = (error, a, b, p0, p1, index)
    _, a, b, p0, p1, index = best
    # 第一个索引的最高位隐含为 0，不满足就交换端点
    if index[0] >= 8:
        a, b, p0, p1 = b, a, p1, p0
        index = 15 - index
    value = MODE_6
    shift = 7
    for channel in range(4):
        for e in (a, b):
            value |= int(e[channel]) << shift
            shift += 7
    value |= p0 << shift
    shift += 1
    value |= p1 << shift
    shift += 1
    value |= int(index[0]) << shift
    shift += 3
    for i in index[1:]:
        value |= int(i) << shift
        shift += 4
    return value.to_bytes(BLOCK_BYTES, "little")


def main():
    if len(sys.argv) not in (3, 4):
        raise SystemExit(__doc__)
    source = open(sys.argv[1], "rb").read()
    if len(source) != (ICON // BLOCK) ** 2 * BLOCK_BYTES:
        raise ValueError(f"expected a {ICON}x{ICON} icon, got {len(source)} bytes")
    base = Image.open(io.BytesIO(dds(source))).convert("RGBA")
    icon = draw_zero(base)
    before = np.asarray(base, dtype=np.uint8)
    after = np.asarray(icon, dtype=np.uint8)
    out = bytearray(source)
    changed = 0
    per_row = ICON // BLOCK
    for row in range(per_row):
        for col in range(per_row):
            y, x = row * BLOCK, col * BLOCK
            if (before[y:y + BLOCK, x:x + BLOCK] == after[y:y + BLOCK, x:x + BLOCK]).all():
                continue
            at = (row * per_row + col) * BLOCK_BYTES
            out[at:at + BLOCK_BYTES] = encode_block(after[y:y + BLOCK, x:x + BLOCK])
            changed += 1
    open(sys.argv[2], "wb").write(out)
    # 解压回来核对，压缩误差过大就报错
    check = np.asarray(Image.open(io.BytesIO(dds(bytes(out)))).convert("RGBA"), dtype=np.float64)
    target = after.astype(np.float64)
    premultiplied = lambda a: np.concatenate((a[:, :, :3] * a[:, :, 3:] / 255.0, a[:, :, 3:]), axis=2)
    worst = np.abs(premultiplied(check) - premultiplied(target)).max()
    print(f"改动 {changed} 块，最大误差 {worst:.0f}")
    if worst > MAX_ERROR:
        raise ValueError("BC7 encode error too large")
    if len(sys.argv) == 4:
        Image.fromarray(check.astype(np.uint8), "RGBA").save(sys.argv[3])


if __name__ == "__main__":
    main()
