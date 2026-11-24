"""CPU-weighted analysis of a samply profile for the ChipDAW app process.

usage: python -I analyze.py <profile.json.gz> <symbol-server-base-url> <out-prefix> [process=app.exe]

Weights every sample by threadCPUDelta (µs of real CPU since the previous
sample), so off-CPU/wait samples from context-switch stacks count ~0.
Writes <out-prefix>.folded (for inferno) and <out-prefix>.txt (report).
"""
import collections
import gzip
import json
import re
import sys
import urllib.request

prof_path, server, out_prefix = sys.argv[1], sys.argv[2].rstrip('/'), sys.argv[3]
proc_name = sys.argv[4] if len(sys.argv) > 4 else 'app.exe'
d = json.load(gzip.open(prof_path))
libs = d['libs']
threads = [t for t in d['threads'] if t.get('processName') == proc_name and t['samples']['length'] > 0]

# ---- symbolication (batched per thread, all frames used) --------------------
def symbolicate(t):
    ft, fu, rt = t['frameTable'], t['funcTable'], t['resourceTable']
    names = {}
    mem, mem_idx, req = [], {}, []
    for fr in range(ft['length']):
        f = ft['func'][fr]
        r = fu['resource'][f]
        addr = ft['address'][fr]
        if r is None or r < 0 or addr is None or addr < 0:
            continue
        lib = libs[rt['lib'][r]]
        key = (lib['debugName'], lib['breakpadId'])
        if key not in mem_idx:
            mem_idx[key] = len(mem)
            mem.append(list(key))
        req.append((fr, mem_idx[key], addr))
    for i in range(0, len(req), 4000):
        chunk = req[i:i + 4000]
        body = json.dumps({'memoryMap': mem, 'stacks': [[[m, a] for _, m, a in chunk]]}).encode()
        rq = urllib.request.Request(server + '/symbolicate/v5', data=body, headers={'Content-Type': 'application/json'})
        res = json.load(urllib.request.urlopen(rq, timeout=600))
        for (fr, m, a), sym in zip(chunk, res['results'][0]['stacks'][0]):
            fn = sym.get('function') or ('0x%x' % a)
            # Rust v0 / legacy hashes: strip ::h0123... suffix
            fn = re.sub(r'::h[0-9a-f]{16}$', '', fn)
            names[fr] = '%s!%s' % (mem[m][0], fn)
    return names

def frame_name(t, names, fr):
    if fr in names:
        return names[fr]
    f = t['frameTable']['func'][fr]
    r = t['funcTable']['resource'][f]
    lib = t['stringArray'][t['resourceTable']['name'][r]] if r is not None and r >= 0 else '?'
    return '%s!%s' % (lib, t['stringArray'][t['funcTable']['name'][f]])

DRIVER = ('nvoglv64', 'nvlddmkm', 'dxgkrnl', 'dxgmms2', 'opengl32', 'nvgpucomp', 'nvspcap', 'gdi32full', 'dxgi', 'd3d11', 'nvwgf2um')

RASTER = ('SkScan', 'Blitter', 'SkDraw::', 'skcpu::', 'SkMask', 'SkBlurMask', 'SkA8', 'SkAAClip', 'SkEdge', 'GrSWMaskHelper',
          'SkRasterPipeline', 'SkScalerContext', 'SkStrike', 'hb_', 'OT::', 'SkTypeface', 'DWrite', 'SkFontMgr')

def is_driver(frame):
    m = frame.split('!')[0].lower()
    return any(d in m for d in DRIVER) or m.startswith(('ntkrnlmp', 'win32k', 'dxg', 'nvlddmkm', 'ntoskrnl'))

def categorize(stack, main_thread=True):
    """stack: frame names root..leaf. Returns one of the Step 1.4 buckets."""
    if not main_thread:
        if any('nvoglv64' in f.split('!')[0].lower() for f in stack[:6]):
            return '5 GL driver worker thread (nvoglv64)'
        return '9 other threads'
    joined = ' | '.join(stack)
    leaf = stack[-1] if stack else '?'
    raster = any(any(r in f for r in RASTER) for f in stack[-12:])
    if re.search(r'wglSwap|!SwapBuffers', joined):
        return '6 present (SwapBuffers)'
    if 'wglMakeCurrent' in joined:
        return '6 make-current'
    if 'GrDirectContext::flush' in joined:
        if is_driver(leaf):
            return '4 GL driver (in Skia flush, main thread)'
        return '3 Skia flush: GPU-backend CPU work (ops prepare, triangulation, uploads)' if not raster else '3b Skia CPU rasterization (in flush)'
    in_ours = any(re.search(r'ui::views::.*draw|ui::views::(clip|gfx)::', f) for f in stack)
    in_vizia_draw = any('vizia_core::systems::draw' in f or 'DrawContext::draw_' in f for f in stack)
    if in_ours or in_vizia_draw:
        who = 'our custom views' if in_ours else 'vizia stylesheet drawing (bg/shadow/border/text)'
        if is_driver(leaf):
            return '4 GL driver (during draw recording, main thread)'
        if raster:
            return '3b Skia CPU rasterization (glyphs/masks) from ' + who
        if in_ours:
            return '1 our code: custom view draw recording (incl. Skia record calls)'
        return '2 vizia draw traversal + stylesheet recording'
    if 'wait_for_messages' in joined or 'MsgWaitForMultipleObjects' in joined or 'GetMessage' in joined:
        return '7 event-loop wait / message dispatch'
    if re.search(r'vizia_core::systems::(style|layout|animation|text)|style_system|layout_system|morphorm', joined):
        return '2b vizia style/layout'
    if re.search(r'ui::|AppState|vizia_core::events|EventManager|process_timers|Binding|vizia_reactive', joined):
        return '1b events, timers, bindings, our model update'
    if is_driver(leaf):
        return '4 GL driver (other, main thread)'
    return '8 other (main thread)'

report = []
main_t = next((t for t in threads if t.get('name') == proc_name), max(threads, key=lambda t: t['samples']['length']))
folded = collections.Counter()
total_cpu = 0.0
cat_total = collections.Counter()
for t in threads:
    names = symbolicate(t)
    s, st = t['samples'], t['stackTable']
    cpu = s.get('threadCPUDelta') or [0] * s['length']
    cache = {}

    def stack_of(si):
        if si in cache:
            return cache[si]
        out = []
        cur = si
        while cur is not None:
            out.append(frame_name(t, names, st['frame'][cur]))
            cur = st['prefix'][cur]
        out.reverse()
        cache[si] = out
        return out

    tcpu = 0.0
    cats = collections.Counter()
    self_fn = collections.Counter()
    incl_fn = collections.Counter()
    for i in range(s['length']):
        w = cpu[i] or 0
        si = s['stack'][i]
        if si is None or w <= 0:
            continue
        stk = stack_of(si)
        tcpu += w
        c = categorize(stk, t is main_t)
        cats[c] += w
        self_fn[stk[-1]] += w
        for f in set(stk):
            incl_fn[f] += w
        folded[';'.join(['T%s' % t['tid']] + [x.replace(';', ':') for x in stk])] += w
    dur = max((max(s['time']) - min(s['time'])) / 1000.0, 1e-3)
    total_cpu += tcpu
    for k, v in cats.items():
        cat_total[k] += v
    report.append('=== thread %s (%s): CPU %.2f s over %.1f s = %.1f%% of one core' % (t['tid'], t.get('name'), tcpu / 1e6, dur, 100 * tcpu / 1e6 / dur))
    for k, v in cats.most_common():
        report.append('   %5.1f%%  %s' % (100 * v / tcpu, k))
    report.append('   -- top self (CPU-weighted):')
    for k, v in self_fn.most_common(25):
        report.append('   %5.1f%%  %s' % (100 * v / tcpu, k[:150]))
    report.append('   -- top inclusive (app frames):')
    for k, v in [(k, v) for k, v in incl_fn.most_common(400) if k.startswith('app')][:40]:
        report.append('   %5.1f%%  %s' % (100 * v / tcpu, k[:150]))

dur_all = max((max(t['samples']['time']) - min(t['samples']['time'])) for t in threads) / 1000.0
head = ['PROCESS %s: total CPU %.2f s over %.1f s = %.1f%% of one core' % (proc_name, total_cpu / 1e6, dur_all, 100 * total_cpu / 1e6 / dur_all),
        'Category breakdown (share of process CPU):']
for k, v in cat_total.most_common():
    head.append('   %5.1f%%  (%5.1f%% core)  %s' % (100 * v / total_cpu, 100 * v / 1e6 / dur_all, k))
open(out_prefix + '.txt', 'w', encoding='utf-8').write('\n'.join(head + [''] + report) + '\n')
with open(out_prefix + '.folded', 'w', encoding='utf-8') as fo:
    for k, v in folded.items():
        fo.write('%s %d\n' % (k, int(v)))
print('\n'.join(head))
