"""Lay out docs/design/mockup-D.dc.html in headless Chrome (1440x400 CSS px)
and write every element's box and every text run's line box as JSON lines.

usage: python measure_mockup.py [out.jsonl]
Uses the support.js shim next to this file (its ?measure mode)."""
import html
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "mockup-boxes.jsonl"
    tmp = os.path.join(tempfile.gettempdir(), "chipdaw-mockup-render")
    os.makedirs(tmp, exist_ok=True)
    shutil.copy(os.path.join(REPO, "docs", "design", "mockup-D.dc.html"), os.path.join(tmp, "mockup.html"))
    shutil.copy(os.path.join(HERE, "support.js"), os.path.join(tmp, "support.js"))
    url = "file:///" + tmp.replace("\\", "/") + "/mockup.html?measure"
    dom = subprocess.run(
        [CHROME, "--headless=new", "--disable-gpu", "--window-size=1440,400",
         "--virtual-time-budget=8000", "--dump-dom", url],
        capture_output=True, text=True, encoding="utf-8").stdout
    m = re.search(r'<pre id="measure">([\s\S]*?)</pre>', dom)
    if not m:
        sys.exit("no measurements in the dumped DOM")
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(html.unescape(m.group(1)) + "\n")
    print(out)


if __name__ == "__main__":
    main()
