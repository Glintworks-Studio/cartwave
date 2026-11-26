"""Checks what at_polish.ps1 saw and saved. PASS/FAIL per check; exit 1 on a failure."""
import json
import os
import re
import sys

from PIL import Image

work = sys.argv[1]
obs = json.load(open(os.path.join(work, "obs.json"), encoding="utf-8-sig"))
results = {}


def o(k, d=None):
    v = obs.get(k, d)
    if v is None:
        return d
    return v if isinstance(v, list) or not isinstance(d, list) else [v]


def load(n):
    p = os.path.join(work, n + ".chipdaw")
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None


def check(sc, cond, msg):
    print(("PASS " if cond else "FAIL ") + f"{sc}: {msg}")
    results.setdefault(sc, []).append(bool(cond))


# The palette (crates/ui/src/theme/palette.rs), for "is it a palette colour".
pal_src = open(os.path.join(os.path.dirname(__file__), "..", "..", "..", "crates", "ui", "src", "theme", "palette.rs"), encoding="utf-8").read()
base = pal_src[pal_src.index("pub const BASE"):pal_src.index("/// The colours in use before")]
PALETTE = {"#" + h.upper() for h in re.findall(r"0x([0-9A-Fa-f]{6})", base)}

if "pl01_title" in obs:
    S = "PL-01"
    a, u, fx = load("pl01_after"), load("pl01_undone"), load("fx_basic")
    col = a and a["lanes"][2].get("color")
    check(S, col is not None and col.upper() in PALETTE and col.upper() != "#7F86E0", f"the triangle takes the picked grid colour: {col}")
    check(S, "*" in str(o("pl01_title", "")), f"the project is marked unsaved: {o('pl01_title')}")
    check(S, u is not None and u["lanes"][2].get("color") == fx["lanes"][2].get("color"), f"one Ctrl+Z restores it: {u and u['lanes'][2].get('color')}")
    others = a and all(a["lanes"][i].get("color") == fx["lanes"][i].get("color") for i in range(8) if i != 2)
    check(S, bool(others), "the other tracks are unchanged (the hover preview was put back)")

if os.path.exists(os.path.join(work, "pl02_after.chipdaw")):
    S = "PL-02"
    a = load("pl02_after")
    cols = [c.get("color") for c in a["clips"] if c["lane"] == 1]
    check(S, len(cols) >= 2 and len(set(cols)) == 1 and cols[0] and cols[0].upper() in PALETTE, f"both selected clips take it: {cols}")

if os.path.exists(os.path.join(work, "pl03_after.chipdaw")):
    S = "PL-03"
    a, fx = load("pl03_after"), load("fx_basic")
    new = [c for c in a["clips"] if c["id"] not in {x["id"] for x in fx["clips"]}]
    check(S, len(new) == 1, f"one clip inserted: {len(new)}")
    c = new[0].get("color") if new else None
    check(S, c is not None and c.upper() in PALETTE, f"it got a palette colour of its own: {c}")

if "pl04_scroll" in obs:
    S = "PL-04"
    lines = o("pl04_scroll", [])
    m = [re.search(r"lane scroll (\d+) max (\d+)", l) for l in lines]
    m = [x for x in m if x]
    check(S, bool(m) and int(m[-1].group(2)) > 0, f"the lanes overflow above the inspector: {lines[-1:] }")
    check(S, bool(m) and int(m[-1].group(1)) > 0, "the wheel over the headers scrolled them")
    b, f = (os.path.join(work, n) for n in ("pl04_before.png", "pl04_after.png"))
    if os.path.exists(b) and os.path.exists(f):
        from PIL import ImageChops
        d = ImageChops.difference(Image.open(b).convert("RGB"), Image.open(f).convert("RGB")).getbbox()
        check(S, d is not None and d[0] < 620, f"the headers and lanes moved (screenshots differ in {d})")

if "pl05_client" in obs:
    S = "PL-05"
    w, h, dpi = o("pl05_client", [])
    k = dpi / 96.0
    lw, lh = round(w / k), round(h / k)
    check(S, (lw, lh) == (1296, 720), f"an 800x600 request opens at the minimum: {lw}x{lh} logical ({w}x{h} px at {dpi} dpi)")
    check(S, lw <= 1536 and lh <= 816, "it fits a 1080p laptop at 125% (1536x816)")

bad = 0
for sc, r in results.items():
    ok = all(r)
    bad += not ok
    print(f"{sc}: {'PASS' if ok else 'FAIL'} ({sum(r)}/{len(r)})")
sys.exit(1 if bad else 0)
