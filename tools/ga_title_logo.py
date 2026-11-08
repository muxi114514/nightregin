"""把 GoldenAge 标志里的「ELDEN RING」换成「NIGHTREIGN」：字母取自原图，缺的 H、T 用原字的部件拼，质感和原字一致。
供 tools\\ga_title.py 的 atlas 步骤调用；坐标都是标志原图（2532x1532）里的像素。
"""
import numpy as np
from PIL import Image
from collections import deque

# 原字母的框（含描边），按「ELDEN RING」顺序；纵向统一
TOP, BOTTOM = 455, 525
OLD = {'E1': (843, 902), 'L': (930, 990), 'D': (1011, 1086), 'E2': (1111, 1169), 'N1': (1195, 1271),
       'R': (1351, 1422), 'I': (1444, 1482), 'N2': (1507, 1584), 'G': (1607, 1681)}
AXIS = 1259          # 树的中轴：树顶细尖和树根末梢都在这一列
GAP = 36             # 字框之间的间距（NIGHTREIGN 连成一个词）
HALF_GAP = 21        # T、R 实心笔画到中轴的距离，两边严格相等（字框带的描边左右不一样宽，不能按字框算）
CORE_ALPHA = 200
CORE_LUM = 95
EDGE = 3             # 抠字时字芯向外扩几像素，带上原字的暗描边
CLEAR_EDGE = 6       # 挖旧字时扩得更多，连外发光一起去掉
HEAL_MARGIN = 16     # 补背景的范围比字带上下各多出这么多，保证洞的上下都有原图可参照
SOFTEN = 3           # 竖向补完后横向柔化的半径，免得出竖条纹
MIRROR_REACH = 90    # 中轴两侧这么宽内是树干，挖空处取对侧镜像的树皮（树大体左右对称）
MIRROR_FEATHER = 3   # 镜像补丁的边缘和插值补色渐变的宽度


def letter_mask(img, box, edge=EDGE):
    """字芯（不透明且够亮）里和字母主体连通的部分，再外扩带上描边。
    碰到字框上下的块一般是树枝；但字母也可能连着一小截树枝，只要不碰字框左右（树干比字框宽）就仍算字母"""
    x0, x1 = box
    sub = img[TOP:BOTTOM, x0:x1]
    core = (sub[:, :, 3] > CORE_ALPHA) & (sub[:, :, :3].mean(axis=2) > CORE_LUM)
    h, w = core.shape
    seen = np.zeros_like(core)
    keep = np.zeros_like(core)
    for sy in range(h):
        for sx in range(w):
            if not core[sy, sx] or seen[sy, sx]:
                continue
            q = deque([(sy, sx)]); seen[sy, sx] = True; pts = []
            while q:
                y, x = q.popleft(); pts.append((y, x))
                for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    ny, nx = y + dy, x + dx
                    if 0 <= ny < h and 0 <= nx < w and core[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = True; q.append((ny, nx))
            ys = [p[0] for p in pts]
            xs = [p[1] for p in pts]
            inside_rows = min(ys) >= 3 and max(ys) <= h - 4
            inside_cols = min(xs) > 0 and max(xs) < w - 1
            if len(pts) >= 12 and (inside_rows or inside_cols):
                for y, x in pts:
                    keep[y, x] = True
    grown = keep.copy()
    for _ in range(edge):
        g = grown.copy()
        g[1:] |= grown[:-1]; g[:-1] |= grown[1:]; g[:, 1:] |= grown[:, :-1]; g[:, :-1] |= grown[:, 1:]
        grown = g
    return grown


def cut(img, box):
    """抠出一个字母：框内 RGBA，只保留字母及描边"""
    x0, x1 = box
    m = letter_mask(img, box)
    out = img[TOP:BOTTOM, x0:x1].copy()
    out[:, :, 3] = out[:, :, 3] * m
    return out


def core_cols(glyph):
    """字母实心笔画（不透明且够亮）的第一列和最后一列，含衬线和横笔末端，不算描边"""
    core = (glyph[:, :, 3] > CORE_ALPHA) & (glyph[:, :, :3].mean(axis=2) > CORE_LUM)
    cols = np.nonzero(core.any(axis=0))[0]
    return cols.min(), cols.max()


def trim(glyph):
    cols = np.nonzero(glyph[:, :, 3].max(axis=0) > 8)[0]
    return glyph[:, cols.min():cols.max() + 1]


def over(dst, src, x):
    """把字母按左上角 x 叠到字带上（直通 alpha）"""
    h, w = src.shape[:2]
    region = dst[:, x:x + w]
    sa = src[:, :, 3:4] / 255.0
    da = region[:, :, 3:4] / 255.0
    oa = sa + da * (1 - sa)
    rgb = np.where(oa > 0, (src[:, :, :3] * sa + region[:, :, :3] * da * (1 - sa)) / np.maximum(oa, 1e-6), 0)
    region[:, :, :3] = rgb
    region[:, :, 3:4] = oa * 255


def build_h(i_glyph, e_glyph, width):
    """H = 两根 I + E 的中横拉长"""
    h = np.zeros((BOTTOM - TOP, width, 4), np.float32)
    iw = i_glyph.shape[1]
    bar = middle_bar(e_glyph)
    span = width - 2 * iw + 12
    bar = np.asarray(Image.fromarray(bar.astype(np.uint8), 'RGBA').resize((span, bar.shape[0]), Image.LANCZOS), np.float32)
    top = bar_row(e_glyph, 'middle')
    over(h[top:top + bar.shape[0]], bar, iw - 6)
    over(h, i_glyph, 0)
    over(h, i_glyph, width - iw)
    return h


def stroke_rows(arm):
    """上横真正的笔画有几行：大半宽度都有字的行（末端衬线往下垂的部分不算）"""
    a = arm[:, :, 3] > 120
    full = np.nonzero(a.sum(axis=1) > a.shape[1] * 0.5)[0]
    return full.max() + 1 if len(full) else 6


CLEAN_SPAN = 10      # 从上横里取多宽一截没有裂纹的平直段来拉长
PIECE_FADE = 4       # 拉长的中段两端透明度渐变的宽度


def straight_part(arm, rows):
    """上横里没有衬线、也没有裂纹的一截：先找笔画以下没有像素的列，再在里面挑最暗的列最亮的那一窗"""
    a = arm[:, :, 3] > 60
    below = a[rows + 1:].any(axis=0) if rows + 1 < arm.shape[0] else np.zeros(arm.shape[1], bool)
    cols = np.nonzero(~below & a[:rows].any(axis=0))[0]
    lo, hi = cols.min() + 2, cols.max() - 2
    lum = arm[1:rows - 1, :, :3].mean(axis=2).mean(axis=0)
    best, start = -1.0, lo
    for x in range(lo, max(lo, hi - CLEAN_SPAN) + 1):
        dip = lum[x:x + CLEAN_SPAN].min()
        if dip > best:
            best, start = dip, x
    return arm[:, start:start + CLEAN_SPAN].copy()


def build_t(i_glyph, e_glyph):
    """T = I 去掉顶衬线 + E 的上横（右半）与它的镜像（左半）+ 竖笔上方补一段平直横笔把两半连起来"""
    arm = top_arm(e_glyph)
    aw = arm.shape[1]
    iw = i_glyph.shape[1]
    width = 2 * aw + iw - 8
    rows = stroke_rows(arm)
    top = bar_row(e_glyph, 'top')
    t = np.zeros((BOTTOM - TOP, width, 4), np.float32)
    stem = i_glyph.copy()
    # 切掉 I 的顶衬线，竖笔顶端塞进横笔下面两行，接口不露缝
    stem[:top + rows - 3, :, 3] = 0
    stem_x = aw - 4
    over(t, stem, stem_x)
    bar = t[top:top + arm.shape[0]]
    over(bar, arm[:, ::-1].copy(), 0)
    over(bar, arm, width - aw)
    # 中段横跨竖笔并压住两半横笔的内端；两端透明度渐变，和下面的横笔融成一条（硬切的端头会透出背景成暗线）
    piece = straight_part(arm, rows)
    span = iw + 20
    piece = np.asarray(Image.fromarray(np.clip(piece, 0, 255).astype(np.uint8), 'RGBA').resize((span, piece.shape[0]), Image.BILINEAR), np.float32)
    ramp = np.minimum(1, np.minimum(np.arange(span) + 1, span - np.arange(span)) / PIECE_FADE)
    piece[:, :, 3] *= ramp[None, :]
    over(bar, piece, stem_x + iw // 2 - span // 2)
    return t


def rows_of(e_glyph):
    """E 的三条横在哪几行：竖笔右侧还有像素的行"""
    a = e_glyph[:, :, 3] > 120
    stem_right = np.nonzero(a.sum(axis=0) > a.shape[0] * 0.6)[0].max() + 2
    right = a[:, stem_right:].sum(axis=1) > 2
    runs, s = [], None
    for y, v in enumerate(right):
        if v and s is None:
            s = y
        if not v and s is not None:
            runs.append((s, y)); s = None
    if s is not None:
        runs.append((s, len(right)))
    return stem_right, runs


def bar_row(e_glyph, which):
    _, runs = rows_of(e_glyph)
    return runs[0][0] - 1 if which == 'top' else runs[len(runs) // 2][0] - 1


def top_arm(e_glyph):
    """E 的上横，从竖笔右侧之外切（带进竖笔边上几列的话，会在横笔下面挂一截暗色竖线）"""
    stem_right, runs = rows_of(e_glyph)
    y0, y1 = runs[0]
    return e_glyph[max(y0 - 1, 0):y1 + 2, stem_right + 1:].copy()


def middle_bar(e_glyph):
    stem_right, runs = rows_of(e_glyph)
    y0, y1 = runs[len(runs) // 2]
    piece = e_glyph[max(y0 - 1, 0):y1 + 2, stem_right:]
    # 去掉末端的衬线，只留平直的一段
    return piece[:, :max(8, int(piece.shape[1] * 0.6))].copy()


def box_blur(a, r):
    """三次盒式模糊近似高斯，沿两个轴各做一遍"""
    out = a
    for _ in range(3):
        for axis in (0, 1):
            pad = [(0, 0)] * out.ndim
            pad[axis] = (r + 1, r)
            c = np.cumsum(np.pad(out, pad, mode='edge'), axis=axis)
            hi = np.take(c, np.arange(2 * r + 1, c.shape[axis]), axis=axis)
            lo = np.take(c, np.arange(0, c.shape[axis] - 2 * r - 1), axis=axis)
            out = (hi - lo) / (2 * r + 1)
    return out


def span_fill(pre, mask):
    """沿第 0 轴在洞的两端之间按预乘颜色线性过渡；返回过渡结果和每个洞像素所在洞段的长度"""
    fill = pre.copy()
    length = np.full(mask.shape, np.inf, np.float32)
    n = mask.shape[0]
    for x in np.nonzero(mask.any(axis=0))[0]:
        col = mask[:, x]
        y = 0
        while y < n:
            if not col[y]:
                y += 1
                continue
            start = y
            while y < n and col[y]:
                y += 1
            before, after = start - 1, y
            if before < 0 or after >= n:
                raise ValueError('hole touches the heal band edge; raise HEAL_MARGIN')
            t = (np.arange(start, y) - before) / (after - before)
            fill[start:y, x] = pre[before, x] * (1 - t)[:, None] + pre[after, x] * t[:, None]
            length[start:y, x] = after - before
    return fill, length


def heal(img, mask):
    """挖掉旧字后补背景：竖向、横向各在洞的两端之间线性过渡，按洞段长度的平方反比混合
    （笔画在窄的方向上补，不出长条纹；树干这种竖直结构也能接上），再在洞里柔化一下"""
    alpha = img[:, :, 3:4] / 255.0
    pre = np.concatenate((img[:, :, :3] * alpha, img[:, :, 3:4]), axis=2)
    vertical, lv = span_fill(pre, mask)
    horizontal, lh = span_fill(pre.transpose(1, 0, 2), mask.T)
    horizontal, lh = horizontal.transpose(1, 0, 2), lh.T
    wv = 1 / np.square(lv); wh = 1 / np.square(lh)
    fill = (vertical * wv[:, :, None] + horizontal * wh[:, :, None]) / np.maximum(wv + wh, 1e-9)[:, :, None]
    fill = np.where(mask[:, :, None], fill, pre)
    soft = box_blur(fill, SOFTEN)
    fill = np.where(mask[:, :, None], soft, fill)
    a = fill[:, :, 3:4] / 255.0
    out = img.copy()
    out[:, :, :3] = np.where(a > 1e-4, fill[:, :, :3] / np.maximum(a, 1e-4), 0)
    out[:, :, 3:4] = fill[:, :, 3:4]
    return np.where(mask[:, :, None], out, img)


def mirror_trunk(original, healed, mask):
    """树干附近的挖空处换成中轴对侧的原图（对侧也被挖了的不换），边缘渐变到插值补色"""
    out = healed.copy()
    xs = np.arange(mask.shape[1])
    mirror = 2 * AXIS - xs
    near = (np.abs(xs - AXIS) < MIRROR_REACH) & (mirror >= 0) & (mirror < mask.shape[1])
    usable = mask & near[None, :]
    usable[:, near] &= ~mask[:, mirror[near]]
    depth = np.zeros(mask.shape, np.float32)
    inner = usable.copy()
    for _ in range(MIRROR_FEATHER):
        depth += inner
        e = inner.copy()
        e[1:] &= inner[:-1]; e[:-1] &= inner[1:]; e[:, 1:] &= inner[:, :-1]; e[:, :-1] &= inner[:, 1:]
        inner = e
    w = (depth / MIRROR_FEATHER)[:, :, None]
    src = np.zeros_like(original)
    src[:, near] = original[:, mirror[near]]
    return np.where(usable[:, :, None], src * w + healed * (1 - w), out)


def relabel(logo):
    """logo：PIL RGBA 标志原图；返回替换文字后的新图"""
    img = np.asarray(logo, np.float32).copy()
    glyphs = {k: trim(cut(img, v)) for k, v in OLD.items()}
    # 挖掉旧字（连描边和外发光），在上下各放宽的区域里补背景
    region = img[TOP - HEAL_MARGIN:BOTTOM + HEAL_MARGIN].copy()
    holes = np.zeros(region.shape[:2], bool)
    for k, (x0, x1) in OLD.items():
        holes[HEAL_MARGIN:HEAL_MARGIN + BOTTOM - TOP, x0:x1] |= letter_mask(img, (x0, x1), CLEAR_EDGE)
    img[TOP - HEAL_MARGIN:BOTTOM + HEAL_MARGIN] = mirror_trunk(region, heal(region, holes), holes)
    band = img[TOP:BOTTOM].copy()
    n_width = glyphs['N1'].shape[1]
    H = trim(build_h(glyphs['I'], glyphs['E1'], n_width - 4))
    T = trim(build_t(glyphs['I'], glyphs['E1']))
    left = [glyphs['N1'], glyphs['I'], glyphs['G'], H, T]
    right = [glyphs['R'], glyphs['E2'], glyphs['I'], glyphs['G'], glyphs['N2']]
    # T 实心部分的最右列和 R 实心部分的最左列关于中轴对称
    t_first, t_last = core_cols(T)
    r_first, _ = core_cols(glyphs['R'])
    left_end = AXIS - HALF_GAP + 1 + (T.shape[1] - 1 - t_last)
    right_start = AXIS + HALF_GAP - r_first
    width = sum(g.shape[1] for g in left) + GAP * (len(left) - 1)
    x = left_end - width
    for g in left:
        over(band, g, x); x += g.shape[1] + GAP
    x = right_start
    for g in right:
        over(band, g, x); x += g.shape[1] + GAP
    img[TOP:BOTTOM] = band
    return Image.fromarray(np.clip(img, 0, 255).astype(np.uint8), 'RGBA')
