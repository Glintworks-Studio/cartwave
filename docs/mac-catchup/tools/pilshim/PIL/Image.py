import struct
import zlib

NEAREST = 0


class _Px:
    def __init__(self, im):
        self.im = im

    def __getitem__(self, xy):
        x, y = xy
        i = (y * self.im.width + x) * 3
        d = self.im._d
        return (d[i], d[i + 1], d[i + 2])

    def __setitem__(self, xy, v):
        x, y = xy
        i = (y * self.im.width + x) * 3
        self.im._d[i:i + 3] = bytes(v[:3])


class Image:
    def __init__(self, w, h, data=None, color=(0, 0, 0)):
        self.width, self.height = w, h
        self.mode = "RGB"
        self._d = bytearray(data) if data is not None else bytearray(bytes(color[:3]) * (w * h))

    @property
    def size(self):
        return (self.width, self.height)

    def convert(self, mode="RGB"):
        return self

    def load(self):
        return _Px(self)

    def getpixel(self, xy):
        return _Px(self)[xy]

    def copy(self):
        return Image(self.width, self.height, self._d)

    def crop(self, box):
        x0, y0, x1, y1 = (int(v) for v in box)
        out = Image(x1 - x0, y1 - y0)
        for y in range(y0, y1):
            if 0 <= y < self.height:
                a, b = max(x0, 0), min(x1, self.width)
                if a < b:
                    out._d[((y - y0) * out.width + (a - x0)) * 3:((y - y0) * out.width + (b - x0)) * 3] = self._d[(y * self.width + a) * 3:(y * self.width + b) * 3]
        return out

    def resize(self, size, resample=NEAREST):
        w, h = size
        out = Image(w, h)
        for y in range(h):
            sy = min(self.height - 1, y * self.height // h)
            for x in range(w):
                sx = min(self.width - 1, x * self.width // w)
                i, j = (sy * self.width + sx) * 3, (y * w + x) * 3
                out._d[j:j + 3] = self._d[i:i + 3]
        return out

    def paste(self, im, xy):
        x0, y0 = xy[0], xy[1]
        for y in range(im.height):
            Y = y0 + y
            if 0 <= Y < self.height:
                a, b = max(x0, 0), min(x0 + im.width, self.width)
                if a < b:
                    self._d[(Y * self.width + a) * 3:(Y * self.width + b) * 3] = im._d[(y * im.width + a - x0) * 3:(y * im.width + b - x0) * 3]

    def point(self, fn):
        lut = bytes(max(0, min(255, int(fn(v)))) for v in range(256))
        return Image(self.width, self.height, bytes(self._d).translate(lut))

    def split(self):
        return [Image(self.width, self.height, bytes(c for c in self._d[k::3] for _ in range(3))) for k in range(3)]

    def save(self, path, *a, **k):
        raw = b"".join(b"\0" + bytes(self._d[y * self.width * 3:(y + 1) * self.width * 3]) for y in range(self.height))
        def chunk(t, d):
            return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
        import builtins
        with builtins.open(path, "wb") as f:
            f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", self.width, self.height, 8, 2, 0, 0, 0))
                    + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def new(mode, size, color=(0, 0, 0)):
    if isinstance(color, int):
        color = (color, color, color)
    return Image(size[0], size[1], color=color)


def open(path):
    import builtins
    b = builtins.open(path, "rb").read()
    assert b[:8] == b"\x89PNG\r\n\x1a\n", path
    pos, idat, pal = 8, [], None
    while pos < len(b):
        n = struct.unpack(">I", b[pos:pos + 4])[0]
        t, d = b[pos + 4:pos + 8], b[pos + 8:pos + 8 + n]
        if t == b"IHDR":
            w, h, bd, ct, _, _, il = struct.unpack(">IIBBBBB", d)
            assert bd == 8 and il == 0, (path, bd, il)
        elif t == b"PLTE":
            pal = d
        elif t == b"IDAT":
            idat.append(d)
        pos += 12 + n
    bpp = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[ct]
    raw = zlib.decompress(b"".join(idat))
    stride = w * bpp
    out = bytearray(h * stride)
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        f = raw[p]
        line = bytearray(raw[p + 1:p + 1 + stride])
        p += 1 + stride
        if f == 1:
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                line[i] = (line[i] + ((line[i - bpp] if i >= bpp else 0) + prev[i]) // 2) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                c = prev[i - bpp] if i >= bpp else 0
                bb = prev[i]
                pa, pb, pc = abs(bb - c), abs(a - c), abs(a + bb - 2 * c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else bb if pb <= pc else c)) & 255
        out[y * stride:(y + 1) * stride] = line
        prev = line
    if ct == 2:
        rgb = out
    elif ct == 6:
        rgb = bytearray(w * h * 3)
        rgb[0::3], rgb[1::3], rgb[2::3] = out[0::4], out[1::4], out[2::4]
    elif ct == 3:
        rgb = bytearray(b"".join(pal[i * 3:i * 3 + 3] for i in out))
    else:
        g = out[0::bpp]
        rgb = bytearray(w * h * 3)
        rgb[0::3], rgb[1::3], rgb[2::3] = g, g, g
    return Image(w, h, rgb)
