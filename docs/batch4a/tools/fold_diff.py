"""Compares two folded-stack files (sample_ui.ps1) by inclusive samples per function.

Rust v0 symbols are reduced to their readable identifier path (crude, enough to tell functions
apart). Prints the functions whose share of all samples grew the most from <base> to <new>.

usage: python fold_diff.py <base.folded> <new.folded> [top]
"""
import re
import sys
from collections import Counter

IDENT = re.compile(r"(\d+)(_?[A-Za-z][A-Za-z0-9_]*)")


def readable(frame):
    mod, _, sym = frame.partition("!")
    if not sym.startswith(("_R", "R")) or "::" in sym:
        return frame[:120]
    parts = []
    i = 0
    while i < len(sym):
        m = IDENT.match(sym, i)
        if m:
            n = int(m.group(1))
            ident = sym[m.start(2):m.start(2) + n]
            if len(ident) == n and not re.fullmatch(r"Cs[0-9A-Za-z_]+", ident):
                parts.append(ident.lstrip("_"))
                i = m.start(2) + n
                continue
        i += 1
    keep = [p for p in parts if not re.fullmatch(r"[0-9a-f]{8,}|llvm|.*\d{5,}.*", p)]
    return mod + "!" + "::".join(keep[-4:])


def load(path):
    inc, total = Counter(), 0
    for line in open(path, encoding="utf-8-sig"):
        line = line.strip()
        if not line:
            continue
        stack, _, n = line.rpartition(" ")
        try:
            n = int(n)
        except ValueError:
            continue
        total += n
        for f in set(readable(f) for f in stack.split(";")):
            inc[f] += n
    return inc, total


def main():
    a, ta = load(sys.argv[1])
    b, tb = load(sys.argv[2])
    top = int(sys.argv[3]) if len(sys.argv) > 3 else 30
    rows = []
    for f in set(a) | set(b):
        pa, pb = a[f] / ta * 100, b[f] / tb * 100
        rows.append((pb - pa, pa, pb, f))
    rows.sort(reverse=True)
    print(f"samples: base {ta}, new {tb}")
    print(f"{'delta':>7} {'base%':>7} {'new%':>7}  function")
    for d, pa, pb, f in rows[:top]:
        print(f"{d:+7.2f} {pa:7.2f} {pb:7.2f}  {f}")


if __name__ == "__main__":
    main()
