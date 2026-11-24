"""CPU breakdown of an Instruments "Time Profiler" trace, in PERF.md's categories.

usage: tp_categories.py <trace> [--process app] [--json]

Only "Running" samples count (Time Profiler samples on-CPU threads), so the
result is CPU time, shown as % of one core over the sampled span, split by P/E
core. Each sample is assigned to a category by its innermost recognizable frame
(see CATEGORIES). Also prints the per-thread split and the top leaf functions.
"""
import collections
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

args = sys.argv[1:]
trace = args[0]
proc_name = args[args.index("--process") + 1] if "--process" in args else "app"

xml = subprocess.run(
    ["xctrace", "export", "--input", trace, "--xpath",
     '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]'],
    capture_output=True, text=True, check=True).stdout
root = ET.fromstring(xml)

ids = {}


def reg(el):
    """Resolve id/ref: returns the canonical element."""
    if "ref" in el.attrib:
        return ids[el.attrib["ref"]]
    if "id" in el.attrib:
        ids[el.attrib["id"]] = el
    for sub in el:
        reg(sub)
    return el


GPU_BINS = ("AppleMetalOpenGLRenderer", "GLEngine", "libGL", "GLRenderer", "OpenGL", "Metal", "AGX",
            "IOGPU", "IOSurface", "libCoreVMClient", "libGFXShared", "libCVMS")
# (category, predicate on (frame name, binary name)) checked innermost-first.
CATEGORIES = [
    ("GL-on-Metal driver (main thread, inside GL/flush calls)", lambda n, b: b.startswith(GPU_BINS)),
    ("present (flushBuffer)", lambda n, b: "flushBuffer" in n or "CGLFlushDrawable" in n),
    ("Skia CPU raster (blur masks, glyphs)", lambda n, b: re.search(r"SkScan|SkBlitter|SkDraw|SkRasterPipeline|SkMaskFilter|SkBlurMask|SkGlyph|SkStrike|SkScalerContext|SkFontMgr|SkTypeface_Mac|CTFont|CGGlyph", n)),
    ("Skia GPU flush (op prepare, triangulation, uploads)", lambda n, b: re.search(r"Gr[A-Z]\w*|skgpu", n) and b == proc_name),
    ("Skia record / other", lambda n, b: re.search(r"Sk[A-Z]\w*", n) and b == proc_name),
    ("our code (custom views, model)", lambda n, b: re.search(r"Cs\w+_2ui", n)),
    ("vizia (traversal, style, layout, text)", lambda n, b: re.search(r"vizia_|morphorm|parley|swash|skrifa|cosmic", n)),
    ("winit / AppKit event loop", lambda n, b: re.search(r"winit", n) or b in ("AppKit", "HIToolbox", "CoreFoundation", "Foundation", "SkyLight", "QuartzCore")),
]

span_ns = 0
first = last = None
by_cat = collections.Counter()
by_cat_core = collections.Counter()
by_thread = collections.Counter()
by_leaf = collections.Counter()
for row in root.iter("row"):
    cells = [reg(c) for c in row]
    rec = {c.tag: c for c in cells}
    t = int(rec["sample-time"].text)
    first = t if first is None else min(first, t)
    last = t if last is None else max(last, t)
    thread = rec["thread"]
    proc = thread.find("process") if thread.find("process") is not None else rec.get("process")
    pname = proc.attrib.get("fmt", "") if proc is not None else ""
    if not pname.startswith(proc_name + " "):
        continue
    state = rec.get("thread-state")
    if state is not None and state.text not in (None, "Running"):
        continue
    w = int(rec["weight"].text)
    core = rec.get("core").attrib.get("fmt", "") if rec.get("core") is not None else ""
    ctype = "P" if "P Core" in core else "E" if "E Core" in core else "?"
    bt = rec.get("tagged-backtrace") if rec.get("tagged-backtrace") is not None else rec.get("backtrace")
    frames = []
    if bt is not None:
        for f in bt.iter("frame"):
            f = ids.get(f.attrib.get("ref"), f)
            b = f.find("binary")
            if b is not None and "ref" in b.attrib:
                b = ids[b.attrib["ref"]]
            frames.append((f.attrib.get("name", ""), b.attrib.get("name", "") if b is not None else ""))
    cat = "other (kernel, libsystem, unresolved)"
    for n, b in frames:
        hit = next((c for c, pred in CATEGORIES if pred(n, b)), None)
        if hit:
            cat = hit
            break
    tname = re.sub(r" \(0x[0-9a-f]+\).*", "", thread.attrib.get("fmt", "?"))
    by_cat[cat] += w
    by_cat_core[(cat, ctype)] += w
    by_thread[tname] += w
    if frames:
        by_leaf[f"{frames[0][0][:90]}  [{frames[0][1]}]"] += w

span = (last - first) if first is not None else 1
pct = lambda w: w / span * 100
total = sum(by_cat.values())
out = {"span_s": span / 1e9, "total_pct": pct(total),
       "categories": {c: {"pct": pct(w), "P": pct(by_cat_core[(c, 'P')]), "E": pct(by_cat_core[(c, 'E')])}
                      for c, w in by_cat.most_common()},
       "threads": {t: pct(w) for t, w in by_thread.most_common(8)}}
if "--json" in args:
    print(json.dumps(out))
    sys.exit()
print(f"span {span / 1e9:.2f}s  total {pct(total):.1f}% of one core")
print(f"{'category':58} {'%core':>6} {'P':>6} {'E':>6}")
for c, v in out["categories"].items():
    print(f"{c:58} {v['pct']:6.1f} {v['P']:6.1f} {v['E']:6.1f}")
print("threads:", {t: round(v, 1) for t, v in out["threads"].items()})
print("top leaf functions:")
for l, w in by_leaf.most_common(15):
    print(f"  {pct(w):5.1f}  {l}")
