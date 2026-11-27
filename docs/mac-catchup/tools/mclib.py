"""Shared helpers for the MAC_CATCHUP Python checks: launch the app with its own data dir,
drive it with macdrive, read saved files, settings, the menu bar and screenshots."""
import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[3]
WORK = pathlib.Path(os.environ.get("MC_WORK", os.path.join(os.environ.get("TMPDIR", "/tmp"), "chipdaw-mac-catchup")))
BIN = WORK / "bin"
BAR = 3840
PX_BAR = 92.0   # default zoom
X0 = 622        # bar 1's line, content x
ROW0, PITCH = 136, 73

ap = argparse.ArgumentParser()
ap.add_argument("--app", default=os.environ.get("MC_APP", str(REPO / "target/release/cartwave")))
ap.add_argument("--only", default="")
ap.add_argument("--out", default="")
args, _rest = ap.parse_known_args()
ONLY = set(filter(None, args.only.split(",")))

for tool in ("macdrive", "img"):
    src = REPO / f"docs/mac-catchup/tools/{tool}.swift"
    out = BIN / tool
    if not out.exists() or src.stat().st_mtime > out.stat().st_mtime:
        BIN.mkdir(parents=True, exist_ok=True)
        subprocess.run(["swiftc", "-O", str(src), "-o", str(out)], check=True, capture_output=True)

results = []


def report(name, ok, detail=""):
    line = f"{'PASS' if ok is True else 'FAIL' if ok is False else ok}  {name}: {detail}"
    print(line, flush=True)
    results.append(line)


def x_of(bar, frac=0.0):
    return X0 + (bar - 1 + frac) * PX_BAR


def y_of(lane, part="body"):
    top = ROW0 + PITCH * lane
    return top + (10 if part == "header" else 44)


class App:
    def __init__(self, name, project=None, env=None, settings=None):
        subprocess.run(["pkill", "-x", "cartwave"], capture_output=True)
        time.sleep(0.5)
        self.name = name
        self.data = WORK / f"data-{name}"
        subprocess.run(["rm", "-rf", str(self.data)])
        self.data.mkdir(parents=True)
        s = {"audio_device": os.environ.get("MC_DEVICE", "Mac mini Speakers")}
        s.update(settings or {})
        (self.data / "settings.json").write_text(json.dumps(s))
        e = dict(os.environ, CARTWAVE_DEV="1", CARTWAVE_DATA_DIR=str(self.data))
        self.proj = None
        if project:
            self.proj = WORK / f"{name}.chipdaw"
            self.proj.write_bytes(pathlib.Path(project).read_bytes())
            e["CARTWAVE_PROJECT"] = str(self.proj)
        e.update(env or {})
        self.log_path = WORK / f"{name}.log"
        self.p = subprocess.Popen([args.app], env=e, stdout=open(self.log_path, "w"), stderr=subprocess.STDOUT)
        for _ in range(60):
            if "window" in self.drv("info"):
                break
            time.sleep(0.2)
        time.sleep(1.5)
        self.drv("focus")
        self.drv("move", 1300, 860)

    def drv(self, *cmd):
        out = subprocess.run([str(BIN / "macdrive"), str(self.p.pid)] + [str(c) for c in cmd], capture_output=True, text=True)
        return out.stdout

    def seq(self, *cmds):
        """Several macdrive commands in one call: seq(("key","d","cmd"), ("sleep",0.2))."""
        flat = []
        for c in cmds:
            if flat:
                flat.append(";")
            flat += [str(x) for x in c]
        return self.drv(*flat)

    def log(self):
        return self.log_path.read_text(errors="replace")

    def mark(self):
        return len(self.log())

    def since(self, m):
        return self.log()[m:]

    def save(self):
        before = self.proj.stat().st_mtime_ns
        self.drv("key", "s", "cmd")
        for _ in range(40):
            time.sleep(0.1)
            if self.proj.stat().st_mtime_ns != before:
                break
        time.sleep(0.2)
        return json.loads(self.proj.read_text())

    def doc(self):
        return json.loads(self.proj.read_text())

    def settings(self):
        return json.loads((self.data / "settings.json").read_text())

    def menu(self, item):
        """(enabled, checked, key) of a menu item, from the menu bar."""
        for line in self.drv("menus").splitlines():
            t = line.strip()
            checked = t.startswith("✓ ")
            t = t[2:] if checked else t
            name = t.split("\t")[0].replace(" [disabled]", "")
            if name == item:
                return ("[disabled]" not in t, checked, t.split("\t")[1] if "\t" in t else "")
        return None

    def title(self):
        return self.drv("title").strip()

    def shot(self, name):
        path = WORK / f"{name}.png"
        self.drv("shot", path)
        return path

    def quit(self):
        self.p.kill()
        self.p.wait()


def img(*a):
    return subprocess.run([str(BIN / "img")] + [str(x) for x in a], capture_output=True, text=True).stdout.strip()


def mean(path, x, y, w, h):
    return float(img("mean", path, x, y + 32, w, h))  # screenshots include the 32 pt title bar


def clips(d, lane=None):
    return sorted([c for c in d["clips"] if lane is None or c["lane"] == lane], key=lambda c: (c["lane"], c["start"]))


def by_name(d, name):
    return [c for c in d["clips"] if c["name"] == name]


def want(name):
    return not ONLY or name in ONLY


