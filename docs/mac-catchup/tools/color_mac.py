#!/usr/bin/env python3
"""MAC_CATCHUP §4.7: are the app's colours tagged sRGB and shown right on both displays?

For each display: the app (a fixture with Pulse, Triangle, Noise and VRC6 clips) and a native
AppKit window of the same sRGB colours (swatch.swift) side by side. Both are captured with
screencapture (tagged with the display's profile) and read two ways:
  * converted to sRGB through that profile: what Digital Color Meter shows in "Display in sRGB";
  * raw: the display-space values the panel receives.
If the GL window is colour-managed as sRGB, its raw values equal the native window's and its
sRGB values equal the tokens. An untagged (device RGB) window would show raw = token instead.

usage: color_mac.py [--out results.md]  (writes docs/screens/mac/mac-color-check.png)
"""
import json
import pathlib
import re
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from mclib import *  # noqa: E402,F401,F403

DISPLAYS = [("GA27T1M (320 Hz, main)", 560, 128), ("XB271HU (60 Hz, second)", 560, -1440 + 128)]
TOKENS = {"pulse": 0xC9D24A, "triangle": 0x7F86E0, "noise": 0xB5AC98, "vrc6": 0xD9579A, "well": 0x151515, "bg": 0x0A0A0A, "lime": 0xC7D74E}
SW_ORDER = ["lime", "pulse", "triangle", "noise", "vrc6", "well", "bg", "text"]
TB = 32
# Flat app areas (content coordinates, 8 x 8 boxes): clip headers (the track colour), the
# spectrum well (#151515, stopped), a track header's background (#0A0A0A).
SPOTS = {
    "pulse": (700, 136 + 0 * 73 + 7),
    "triangle": (900, 136 + 2 * 73 + 7),
    "noise": (1060, 136 + 3 * 73 + 7),
    "vrc6": (1060, 136 + 5 * 73 + 7),
    "well": (1200, 26),
    "bg": (440, 136 + 60),
}

fx = WORK / "color_fixture.chipdaw"
d = json.loads((WORK / "fixture.chipdaw").read_text()) if (WORK / "fixture.chipdaw").exists() else None
if d is None:
    subprocess.run([sys.executable, str(REPO / "docs/mac-catchup/tools/fixtures_mac.py"), str(WORK / "fixture.chipdaw")], check=True)
    d = json.loads((WORK / "fixture.chipdaw").read_text())
e = json.loads(json.dumps(d["clips"][3]))
e.update(id=9, lane=5, name="E")
for n in e["notes"]:
    n["id"] += 1000
d["clips"].append(e)
fx.write_text(json.dumps(d))

subprocess.run(["swiftc", "-O", str(REPO / "docs/mac-catchup/tools/swatch.swift"), "-o", str(BIN / "swatch")], check=True, capture_output=True)
winctl = BIN / "winctl"
if not winctl.exists():
    subprocess.run(["swiftc", "-O", str(REPO / "docs/engine/tools/macos/winctl.swift"), "-o", str(winctl)], check=True, capture_output=True)


def hexv(s):
    return int(s[1:], 16)


def dist(a, b):
    return max(abs(((a >> k) & 255) - ((b >> k) & 255)) for k in (16, 8, 0))


lines = ["| Display | Colour | Token (sRGB) | App, sRGB (DCM \"sRGB\") | App, raw display | Native window, raw | App vs native (max channel) | sRGB vs token |", "|---|---|---|---|---|---|---|---|"]
shots = []
worst = 0
for name, x, y in DISPLAYS:
    a = App("color", fx)
    subprocess.run([str(winctl), str(a.p.pid), "move", str(x), str(y)], capture_output=True)
    import time
    time.sleep(2.5)
    a.drv("move", 1300, 880)
    info = a.drv("info")
    shot = a.shot(f"color_{'320' if y > 0 else '60'}")
    sw = subprocess.Popen([str(BIN / "swatch"), str(x), str(y + 960), "8"], stdout=subprocess.PIPE, text=True)
    time.sleep(2)
    swp = WORK / f"swatch_{'320' if y > 0 else '60'}.png"
    subprocess.run(["screencapture", "-x", f"-R{x},{y + 960},512,64", str(swp)])
    sw.wait()
    lime_mode = None
    # The display's lit indicators ("48k", "16-bit"): the most common bright lime-ish pixel.
    from collections import Counter
    cnt = Counter()
    for yy in range(42, 56):
        row = img("row", shot, yy + TB, 418, 490).splitlines()
        for r in row:
            v = hexv(r.split()[1])
            R, G, B = v >> 16, (v >> 8) & 255, v & 255
            if G > 180 and B < 120:
                cnt[v] += 1
    if cnt:
        lime_mode = cnt.most_common(1)[0][0]
    for key, tok in TOKENS.items():
        si = SW_ORDER.index(key)
        swr = re.search(r"raw (#\w+)", img("srgb", swp, si * 64 + 24, 24, 16, 16)).group(1)
        if key == "lime":
            app_s = f"#{lime_mode:06X} (mode of the indicator text)" if lime_mode else "-"
            app_r = "-"
            d1 = "-"
            d2 = dist(lime_mode, tok) if lime_mode else 99
        else:
            sx, sy = SPOTS[key]
            out = img("srgb", shot, sx, sy + TB, 8, 8)
            app_s = re.search(r"sRGB (#\w+)", out).group(1)
            app_r = re.search(r"raw (#\w+)", out).group(1)
            d1 = dist(hexv(app_r), hexv(swr))
            d2 = dist(hexv(app_s), tok)
        worst = max(worst, d2 if isinstance(d2, int) else 0)
        lines.append(f"| {name} | {key} | #{tok:06X} | {app_s} | {app_r} | {swr} | {d1} | {d2} |")
    shots.append((shot, swp, name, info.strip()))
    a.quit()

report = "\n".join(lines) + f"\n\nWorst sRGB-vs-token difference: {worst} level(s).\n"
print(report)
for s, _, n, i in shots:
    print(n, "->", i)
out_dir = REPO / "docs/screens/mac"
out_dir.mkdir(parents=True, exist_ok=True)
parts = []
for k, (s, swp, n, _) in enumerate(shots):
    c = WORK / f"cc_{k}.png"
    img("crop", s, c, 380, TB, 1060, 520)
    l1 = WORK / f"ccl_{k}.png"
    img("label", c, l1, f"App on {n}")
    l2 = WORK / f"ccs_{k}.png"
    img("label", swp, l2, f"Native sRGB swatches on {n}: lime, pulse, triangle, noise, VRC6, well, bg, text")
    parts += [l1, l2]
img("stack", "v", out_dir / "mac-color-check.png", *parts)
if "--out" in sys.argv:
    pathlib.Path(sys.argv[sys.argv.index("--out") + 1]).write_text(report)
