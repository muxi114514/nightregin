"""Scaleform GFX（SWF 变体，文件头 GFX，未压缩）的读写：标签原样保留，只解析要改的几种。
供 tools\\ga_title.py 使用；也能单独运行打印结构：gfx_swf.py <文件.gfx>
"""
import struct
import sys

TAG_END = 0
TAG_SHOWFRAME = 1
TAG_SHAPE = (2, 22, 32, 83)
TAG_PLACE2 = 26
TAG_REMOVE2 = 28
TAG_LABEL = 43
TAG_SPRITE = 39
TAG_PLACE3 = 70
TAG_SYMBOLS = 76
TAG_EXTERNAL_IMAGE = 1009
TWIPS = 20


class Bits:
    def __init__(self, data, pos=0):
        self.data, self.pos, self.bit = data, pos, 0

    def u(self, n):
        v = 0
        for _ in range(n):
            v = (v << 1) | ((self.data[self.pos] >> (7 - self.bit)) & 1)
            self.bit += 1
            if self.bit == 8:
                self.bit = 0
                self.pos += 1
        return v

    def s(self, n):
        v = self.u(n)
        return v - (1 << n) if n and v >> (n - 1) else v

    def end(self):
        return self.pos + (1 if self.bit else 0)


class BitWriter:
    def __init__(self):
        self.out = bytearray()
        self.cur = 0
        self.n = 0

    def put(self, value, bits):
        for i in range(bits - 1, -1, -1):
            self.cur = (self.cur << 1) | ((value >> i) & 1)
            self.n += 1
            if self.n == 8:
                self.out.append(self.cur)
                self.cur = self.n = 0

    def bytes(self):
        if self.n:
            self.out.append(self.cur << (8 - self.n))
            self.cur = self.n = 0
        return bytes(self.out)


def signed_bits(*values):
    need = 1
    for v in values:
        n = 1
        while not (-(1 << (n - 1)) <= v < (1 << (n - 1))):
            n += 1
        need = max(need, n)
    return need


def read_matrix(data, pos):
    r = Bits(data, pos)
    sx = sy = 1.0
    r0 = r1 = 0.0
    if r.u(1):
        n = r.u(5)
        sx = r.s(n) / 65536
        sy = r.s(n) / 65536
    if r.u(1):
        n = r.u(5)
        r0 = r.s(n) / 65536
        r1 = r.s(n) / 65536
    n = r.u(5)
    tx = r.s(n)
    ty = r.s(n)
    return [sx, sy, r0, r1, tx, ty], r.end()


def write_matrix(m):
    sx, sy, r0, r1, tx, ty = m
    w = BitWriter()
    if (sx, sy) != (1.0, 1.0):
        a, b = round(sx * 65536), round(sy * 65536)
        n = signed_bits(a, b)
        w.put(1, 1); w.put(n, 5); w.put(a & ((1 << n) - 1), n); w.put(b & ((1 << n) - 1), n)
    else:
        w.put(0, 1)
    if (r0, r1) != (0.0, 0.0):
        a, b = round(r0 * 65536), round(r1 * 65536)
        n = signed_bits(a, b)
        w.put(1, 1); w.put(n, 5); w.put(a & ((1 << n) - 1), n); w.put(b & ((1 << n) - 1), n)
    else:
        w.put(0, 1)
    n = signed_bits(tx, ty) if (tx or ty) else 0
    w.put(n, 5)
    if n:
        w.put(tx & ((1 << n) - 1), n); w.put(ty & ((1 << n) - 1), n)
    return w.bytes()


def read_rect(data, pos):
    r = Bits(data, pos)
    n = r.u(5)
    return [r.s(n) for _ in range(4)], r.end()


def write_rect(x0, x1, y0, y1):
    n = signed_bits(x0, x1, y0, y1)
    w = BitWriter()
    w.put(n, 5)
    for v in (x0, x1, y0, y1):
        w.put(v & ((1 << n) - 1), n)
    return w.bytes()


def cstr(data, pos):
    end = data.index(0, pos)
    return data[pos:end].decode('utf-8', 'replace'), end + 1


class Tag:
    def __init__(self, code, body, long_form=False):
        self.code, self.body, self.long_form = code, bytes(body), long_form

    def encode(self):
        size = len(self.body)
        # 原文件用长格式的保持长格式，否则往返不一致
        if size >= 0x3F or self.long_form:
            return struct.pack('<HI', (self.code << 6) | 0x3F, size) + self.body
        return struct.pack('<H', (self.code << 6) | size) + self.body


def parse_tags(data, pos, end):
    out = []
    while pos + 2 <= end:
        head = struct.unpack_from('<H', data, pos)[0]
        pos += 2
        code, size = head >> 6, head & 0x3F
        long_form = size == 0x3F
        if long_form:
            size = struct.unpack_from('<I', data, pos)[0]
            pos += 4
        out.append(Tag(code, data[pos:pos + size], long_form))
        pos += size
        if code == TAG_END:
            break
    return out


class Gfx:
    def __init__(self, data):
        if data[:3] != b'GFX':
            raise ValueError('not an uncompressed GFX file')
        self.version = data[3]
        rect, pos = read_rect(data, 8)
        self.rect_bytes = data[8:pos]
        self.frame_rate, self.frame_count = struct.unpack_from('<HH', data, pos)
        self.tags = parse_tags(data, pos + 4, len(data))

    def encode(self):
        body = self.rect_bytes + struct.pack('<HH', self.frame_rate, self.frame_count) + b''.join(t.encode() for t in self.tags)
        return b'GFX' + bytes((self.version,)) + struct.pack('<I', 8 + len(body)) + body

    def char_id(self, tag):
        if tag.code in TAG_SHAPE or tag.code == TAG_SPRITE:
            return struct.unpack_from('<H', tag.body, 0)[0]
        if tag.code == TAG_EXTERNAL_IMAGE:
            return struct.unpack_from('<I', tag.body, 0)[0]
        return None

    def max_char(self):
        return max((self.char_id(t) or 0) for t in self.tags)


def external_image(tag):
    cid, fmt, w, h = struct.unpack_from('<IHHH', tag.body, 0)
    name_len = tag.body[10]
    name = tag.body[11:11 + name_len].decode('latin-1')
    file_len = tag.body[11 + name_len]
    file = tag.body[12 + name_len:12 + name_len + file_len].decode('latin-1')
    return {'id': cid, 'format': fmt, 'width': w, 'height': h, 'name': name, 'file': file}


def make_external_image(cid, width, height, name, fmt=13):
    exp = name.encode('latin-1')
    file = (name + '.tga').encode('latin-1')
    return Tag(TAG_EXTERNAL_IMAGE, struct.pack('<IHHH', cid, fmt, width, height) + bytes((len(exp),)) + exp + bytes((len(file),)) + file, True)


def decode_place(tag):
    b = tag.body
    if tag.code == TAG_PLACE2:
        flags, flags2, p = b[0], 0, 1
    else:
        flags, flags2, p = b[0], b[1], 2
    out = {'flags': flags, 'flags2': flags2, 'depth': struct.unpack_from('<H', b, p)[0]}
    p += 2
    # 这一版 GFX 带「直接放图片」标记时不写类名，只有类名标记才有
    if tag.code == TAG_PLACE3 and flags2 & 0x08:
        out['class'], p = cstr(b, p)
    if flags & 0x02:
        out['char'] = struct.unpack_from('<H', b, p)[0]
        p += 2
    if flags & 0x04:
        out['matrix'], p = read_matrix(b, p)
    if flags & 0x08:
        r = Bits(b, p)
        add, mul, n = r.u(1), r.u(1), r.u(4)
        m = [r.s(n) for _ in range(4)] if mul else None
        a = [r.s(n) for _ in range(4)] if add else None
        out['cxform'] = (m, a)
        p = r.end()
    if flags & 0x10:
        out['ratio'] = struct.unpack_from('<H', b, p)[0]
        p += 2
    if flags & 0x20:
        out['name'], p = cstr(b, p)
    if flags & 0x40:
        out['clip_depth'] = struct.unpack_from('<H', b, p)[0]
        p += 2
    out['rest'] = b[p:]
    return out


def make_place2(depth, char=None, matrix=None, name=None, move=False):
    flags = (0x01 if move else 0) | (0x02 if char is not None else 0) | (0x04 if matrix else 0) | (0x20 if name else 0)
    body = bytes((flags,)) + struct.pack('<H', depth)
    if char is not None:
        body += struct.pack('<H', char)
    if matrix:
        body += write_matrix(matrix)
    if name:
        body += name.encode('utf-8') + b'\0'
    return Tag(TAG_PLACE2, body)


def make_bitmap_shape(cid, bitmap, width, height):
    """位图填满的矩形（形状 1 型）：左上角在原点，单位像素"""
    w, h = width * TWIPS, height * TWIPS
    fill = bytes((1, 0x41)) + struct.pack('<H', bitmap) + write_matrix([float(TWIPS), float(TWIPS), 0.0, 0.0, 0, 0])
    lines = bytes((0,))
    bw = BitWriter()
    bw.put(1, 4)  # 填充索引位数
    bw.put(0, 4)  # 线条索引位数
    # 移到原点并选填充 1
    bw.put(0, 1); bw.put(0b00011, 5); bw.put(1, 5); bw.put(0, 1); bw.put(0, 1); bw.put(1, 1)
    for dx, dy in ((w, 0), (0, h), (-w, 0), (0, -h)):
        n = signed_bits(dx, dy)
        bw.put(1, 1); bw.put(1, 1); bw.put(max(n, 2) - 2, 4)
        n = max(n, 2)
        if dx and not dy:
            bw.put(0, 1); bw.put(0, 1); bw.put(dx & ((1 << n) - 1), n)
        else:
            bw.put(0, 1); bw.put(1, 1); bw.put(dy & ((1 << n) - 1), n)
    bw.put(0, 6)
    body = struct.pack('<H', cid) + write_rect(0, w, 0, h) + fill + lines + bw.bytes()
    return Tag(2, body)


def describe(gfx, tags=None, indent='  ', images=None):
    tags = gfx.tags if tags is None else tags
    images = images or {}
    frame = 0
    for t in tags:
        if t.code == TAG_EXTERNAL_IMAGE:
            info = external_image(t)
            images[info['id']] = info['name']
            print(f"{indent}图片 {info['id']} {info['name']} {info['width']}x{info['height']}")
        elif t.code in TAG_SHAPE:
            print(f'{indent}形状 {gfx.char_id(t)} 类型{t.code} 长{len(t.body)}')
        elif t.code == TAG_SPRITE:
            sid, frames = struct.unpack_from('<HH', t.body, 0)
            print(f'{indent}剪辑 {sid} {frames}帧')
            describe(gfx, parse_tags(t.body, 4, len(t.body)), indent + '    ', images)
        elif t.code in (TAG_PLACE2, TAG_PLACE3):
            pl = decode_place(t)
            m = pl.get('matrix')
            ms = f" 矩阵 缩({m[0]:.3f},{m[1]:.3f}) 位({m[4] / TWIPS:.1f},{m[5] / TWIPS:.1f})" if m else ''
            cx = f" 色变{pl['cxform']}" if 'cxform' in pl else ''
            print(f"{indent}@{frame} 摆放{t.code} 深{pl['depth']} {'移' if pl['flags'] & 1 else '放'} 角{pl.get('char')} 类{pl.get('class')} 名{pl.get('name')}{ms}{cx} 余{pl['rest'].hex()[:16]}")
        elif t.code == TAG_SHOWFRAME:
            frame += 1
        elif t.code == TAG_LABEL:
            print(f'{indent}@{frame} 标签 {cstr(t.body, 0)[0]}')
        elif t.code == TAG_REMOVE2:
            print(f"{indent}@{frame} 移除 深{struct.unpack_from('<H', t.body, 0)[0]}")
        elif t.code not in (TAG_END,):
            print(f'{indent}标签 {t.code} 长{len(t.body)}')


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    for path in sys.argv[1:]:
        data = open(path, 'rb').read()
        g = Gfx(data)
        print('==', path, '往返', g.encode() == data)
        describe(g)
