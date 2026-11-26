"""Which characters the bundled fonts map (their `cmap` table), without fontTools.
Usage: python cmap_check.py FONT.ttf... -- prints, for each font, whether each probe
character has a glyph. BATCH4A_PROMPT.md §3.3 ("▸" isn't in Inter)."""
import struct
import sys

PROBES = {"▸ U+25B8": 0x25B8, "▶ U+25B6": 0x25B6, "› U+203A": 0x203A, "> U+003E": 0x3E, "→ U+2192": 0x2192, "− U+2212": 0x2212, "… U+2026": 0x2026, "🔒 U+1F512": 0x1F512}


def cmap(path):
    b = open(path, "rb").read()
    n = struct.unpack(">H", b[4:6])[0]
    tables = {b[12 + 16 * i:16 + 16 * i].decode("latin1"): struct.unpack(">II", b[20 + 16 * i:28 + 16 * i]) for i in range(n)}
    off = tables["cmap"][0]
    count = struct.unpack(">H", b[off + 2:off + 4])[0]
    chars = set()
    for i in range(count):
        pid, eid, sub = struct.unpack(">HHI", b[off + 4 + 8 * i:off + 12 + 8 * i])
        s = off + sub
        fmt = struct.unpack(">H", b[s:s + 2])[0]
        if fmt == 4:
            segx2 = struct.unpack(">H", b[s + 6:s + 8])[0]
            segs = segx2 // 2
            ends = struct.unpack(f">{segs}H", b[s + 14:s + 14 + segx2])
            starts = struct.unpack(f">{segs}H", b[s + 16 + segx2:s + 16 + 2 * segx2])
            deltas = struct.unpack(f">{segs}h", b[s + 16 + 2 * segx2:s + 16 + 3 * segx2])
            ro_at = s + 16 + 3 * segx2
            ros = struct.unpack(f">{segs}H", b[ro_at:ro_at + segx2])
            for k in range(segs):
                for c in range(starts[k], ends[k] + 1):
                    if c == 0xFFFF:
                        continue
                    if ros[k] == 0:
                        g = (c + deltas[k]) & 0xFFFF
                    else:
                        a = ro_at + 2 * k + ros[k] + 2 * (c - starts[k])
                        g = struct.unpack(">H", b[a:a + 2])[0]
                        if g:
                            g = (g + deltas[k]) & 0xFFFF
                    if g:
                        chars.add(c)
        elif fmt == 12:
            ngroups = struct.unpack(">I", b[s + 12:s + 16])[0]
            for k in range(ngroups):
                a, z, g = struct.unpack(">III", b[s + 16 + 12 * k:s + 28 + 12 * k])
                chars.update(range(a, z + 1))
    return chars


for f in sys.argv[1:]:
    c = cmap(f)
    print(f.replace("\\", "/").split("/")[-1], ", ".join(f"{k}: {'yes' if v in c else 'NO'}" for k, v in PROBES.items()))
