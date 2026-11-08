"""把 Noto Serif SC 可变字体裁成「只含页面用到的字」的 WOFF。
做法：保留全部字形编号，只清空用不到的字形轮廓（glyf）和变体数据（gvar），cmap / hmtx 等按编号索引的表原样保留；
去掉 GSUB（替换可能指向被清空的字形）。用法：python subset_font.py <源 ttf> <index.html> <输出 woff>"""
import bisect, struct, sys, zlib

src, html, out = sys.argv[1:4]
f = open(src, 'rb').read()
num = struct.unpack('>H', f[4:6])[0]
T = {}
for i in range(num):
    tag, _, off, ln = struct.unpack('>4sIII', f[12 + 16 * i:28 + 16 * i])
    T[tag.decode('latin1')] = f[off:off + ln]

text = open(html, encoding='utf-8').read()
cps = sorted({ord(c) for c in text if ord(c) >= 0x20} | set(range(0x20, 0x7f)) | {0x3000})

# cmap 格式 12：码位 → 字形编号
cm = T['cmap']
off12 = None
for i in range(struct.unpack('>H', cm[2:4])[0]):
    _, _, o = struct.unpack('>HHI', cm[4 + 8 * i:12 + 8 * i])
    if struct.unpack('>H', cm[o:o + 2])[0] == 12:
        off12 = o
        break
assert off12 is not None, 'cmap 里没有格式 12 子表'
ng = struct.unpack('>I', cm[off12 + 12:off12 + 16])[0]
groups = [struct.unpack('>III', cm[off12 + 16 + 12 * g:off12 + 28 + 12 * g]) for g in range(ng)]
starts = [g[0] for g in groups]
keep = {0}
for cp in cps:
    k = bisect.bisect_right(starts, cp) - 1
    if k >= 0 and groups[k][0] <= cp <= groups[k][1]:
        keep.add(groups[k][2] + cp - groups[k][0])

# glyf / loca（长偏移）；复合字形把部件一起保留
nglyph = struct.unpack('>H', T['maxp'][4:6])[0]
assert struct.unpack('>h', T['head'][50:52])[0] == 1, '只处理长 loca'
loca = struct.unpack('>%dI' % (nglyph + 1), T['loca'][:4 * (nglyph + 1)])
glyf = T['glyf']
todo = list(keep)
while todo:
    g = todo.pop()
    d = glyf[loca[g]:loca[g + 1]]
    if len(d) < 10 or struct.unpack('>h', d[:2])[0] >= 0:
        continue
    p = 10
    while True:
        flags, comp = struct.unpack('>HH', d[p:p + 4])
        if comp not in keep:
            keep.add(comp)
            todo.append(comp)
        p += 4 + (4 if flags & 1 else 2)
        p += 2 if flags & 8 else 4 if flags & 0x40 else 8 if flags & 0x80 else 0
        if not flags & 0x20:
            break

pad = lambda b: b + b'\0' * (-len(b) % 4)
ng_out, nl = [], [0]
for g in range(nglyph):
    d = pad(glyf[loca[g]:loca[g + 1]]) if g in keep else b''
    ng_out.append(d)
    nl.append(nl[-1] + len(d))
T['glyf'] = b''.join(ng_out)
T['loca'] = struct.pack('>%dI' % len(nl), *nl)

# gvar：改成长偏移，只保留用到的字形的变体数据
gv = T['gvar']
major, minor, axc, stc, sto, gc, flags, dao = struct.unpack('>HHHHIHHI', gv[:20])
if flags & 1:
    offs = struct.unpack('>%dI' % (gc + 1), gv[20:20 + 4 * (gc + 1)])
else:
    offs = [x * 2 for x in struct.unpack('>%dH' % (gc + 1), gv[20:20 + 2 * (gc + 1)])]
shared = gv[sto:sto + axc * 2 * stc]
datas, no = [], [0]
for g in range(gc):
    d = pad(gv[dao + offs[g]:dao + offs[g + 1]]) if g in keep else b''
    datas.append(d)
    no.append(no[-1] + len(d))
nsto = 20 + 4 * (gc + 1)
ndao = nsto + len(shared) + (-len(shared) % 4)
T['gvar'] = (struct.pack('>HHHHIHHI', major, minor, axc, stc, nsto, gc, flags | 1, ndao)
             + struct.pack('>%dI' % len(no), *no) + pad(shared) + b''.join(datas))
T.pop('GSUB', None)

# cmap 重写成只含保留字的格式 4（每个码位一段）；原字体的格式 12 子表会被浏览器的字体清洗器丢掉，汉字就回落到宋体
cmap_map = {}
for cp in cps:
    k = bisect.bisect_right(starts, cp) - 1
    if cp <= 0xFFFF and k >= 0 and groups[k][0] <= cp <= groups[k][1]:
        cmap_map[cp] = groups[k][2] + cp - groups[k][0]
segs = sorted(cmap_map) + [0xFFFF]
sc = len(segs)
sr = 2 * 2 ** (sc.bit_length() - 1)
f4 = struct.pack('>HHHHHHH', 4, 16 + 8 * sc, 0, 2 * sc, sr, sr.bit_length() - 2, 2 * sc - sr)
f4 += struct.pack('>%dH' % sc, *segs) + b'\0\0' + struct.pack('>%dH' % sc, *segs)
f4 += struct.pack('>%dH' % sc, *[(cmap_map[c] - c) % 65536 if c in cmap_map else 1 for c in segs]) + b'\0\0' * sc
T['cmap'] = struct.pack('>HH', 0, 2) + struct.pack('>HHI', 0, 3, 20) + struct.pack('>HHI', 3, 1, 20) + f4

# 校验和与 head.checkSumAdjustment
def csum(b):
    b = pad(b)
    return sum(struct.unpack('>%dI' % (len(b) // 4), b)) & 0xFFFFFFFF

head = bytearray(T['head'])
head[8:12] = b'\0\0\0\0'
T['head'] = bytes(head)
tags = sorted(T)
n = len(tags)
sfnt_size = 12 + 16 * n + sum(len(pad(T[t])) for t in tags)
es = max(k for k in range(16) if 2 ** k <= n)
hdr = struct.pack('>IHHHH', 0x00010000, n, 16 * 2 ** es, es, 16 * n - 16 * 2 ** es)
o = 12 + 16 * n
dirs = b''
for t in tags:
    dirs += struct.pack('>4sIII', t.encode('latin1'), csum(T[t]), o, len(T[t]))
    o += len(pad(T[t]))
total = (csum(hdr + dirs) + sum(csum(T[t]) for t in tags)) & 0xFFFFFFFF
head[8:12] = struct.pack('>I', (0xB1B0AFBA - total) & 0xFFFFFFFF)
T['head'] = bytes(head)

# WOFF 1.0：每张表 zlib 压缩，压不小就原样存
entries, blobs = [], []
o = 44 + 20 * n
for t in tags:
    raw = T[t]
    z = zlib.compress(raw, 9)
    blob = z if len(z) < len(raw) else raw
    entries.append(struct.pack('>4sIIII', t.encode('latin1'), o, len(blob), len(raw), csum(raw)))
    blobs.append(pad(blob))
    o += len(pad(blob))
whdr = struct.pack('>4sIIHHIHHIIIII', b'wOFF', 0x00010000, o, n, 0, sfnt_size, 1, 0, 0, 0, 0, 0, 0)
open(out, 'wb').write(whdr + b''.join(entries) + b''.join(blobs))
print('chars:', len(cps), 'glyphs kept:', len(keep), 'woff bytes:', o)
