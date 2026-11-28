"""Batch 5's file checks for the AU scenarios (at_b5.ps1): what a saved project's automation is.

usage:
  python check_b5.py points <file> <track index> <param>      -> "n=<count> <t:v:c> ..."
  python check_b5.py at <file> <track index> <param> <tick>...  -> the envelope's value at each tick
  python check_b5.py lanes <file> <track index>                 -> the lanes the track shows
Track index: the track's place in the file (-1: Main). Values are the file's (parameter units).
"""
import json
import sys


def shape(x, c):
    c = max(-0.98, min(0.98, c))
    if c == 0:
        return x
    k = (1 - c) / (1 + c)
    d = x + (1 - x) * k
    return 0.0 if d <= 0 else x / d


def value_at(pts, t):
    if not pts:
        return None
    i = 0
    while i < len(pts) and pts[i][0] <= t:
        i += 1
    if i == 0:
        return pts[0][1]
    a = pts[i - 1]
    if i == len(pts):
        return a[1]
    b = pts[i]
    span = b[0] - a[0]
    if span <= 0:
        return b[1]
    x = min(1.0, max(0.0, (t - a[0]) / span))
    return a[1] + (b[1] - a[1]) * shape(x, a[2])


def envelope(path, track, param):
    j = json.load(open(path, encoding="utf-8"))
    tid = None if track < 0 else j["tracks"][track]["id"]
    for e in j.get("automation", []):
        if e.get("track") == tid and e["param"] == param:
            return e["points"]
    return []


def main():
    cmd, path = sys.argv[1], sys.argv[2]
    if cmd == "points":
        pts = envelope(path, int(sys.argv[3]), sys.argv[4])
        print(f"n={len(pts)} " + " ".join(f"{t}:{v:g}:{c:g}" for t, v, c in pts))
    elif cmd == "at":
        pts = envelope(path, int(sys.argv[3]), sys.argv[4])
        print(" ".join("none" if value_at(pts, int(t)) is None else f"{value_at(pts, int(t)):.3f}" for t in sys.argv[5:]))
    elif cmd == "lanes":
        j = json.load(open(path, encoding="utf-8"))
        track = int(sys.argv[3])
        tid = None if track < 0 else j["tracks"][track]["id"]
        for l in j.get("auto_lanes", []):
            if l.get("track") == tid:
                print(" ".join(l["params"]))
                return
        print("")


if __name__ == "__main__":
    main()
