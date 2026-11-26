"""Lay out a Polish 2 reference board (docs/design/polish2-ref/*.html) in headless
Chrome and write every element's box (and each text node's line box) as JSON lines.

usage: python measure_ref.py E1-header [out.jsonl]

Each line: {"i": n, "tag", "depth", "x", "y", "w", "h", "text"?, "aria"?, "style"?}
Coordinates are CSS px from the board's top-left (1 CSS px = 1 logical px)."""
import html
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

PROBE = r"""
<script>
window.addEventListener('load', async () => {
  try { await document.fonts.ready; } catch (e) {}
  const out = [];
  const root = document.body.firstElementChild;
  const r0 = root.getBoundingClientRect();
  let i = 0;
  function walk(el, depth) {
    const r = el.getBoundingClientRect();
    const rec = {i: i++, tag: el.tagName, depth, x: +(r.left - r0.left).toFixed(2), y: +(r.top - r0.top).toFixed(2),
                 w: +r.width.toFixed(2), h: +r.height.toFixed(2)};
    const a = el.getAttribute('aria-label'); if (a) rec.aria = a;
    const st = el.getAttribute('style'); if (st) rec.style = st;
    let own = '';
    for (const n of el.childNodes) if (n.nodeType === 3) own += n.textContent;
    own = own.trim();
    if (own) {
      rec.text = own;
      for (const n of el.childNodes) if (n.nodeType === 3 && n.textContent.trim()) {
        const rg = document.createRange(); rg.selectNodeContents(n);
        const b = rg.getBoundingClientRect();
        rec.tx = +(b.left - r0.left).toFixed(2); rec.ty = +(b.top - r0.top).toFixed(2);
        rec.tw = +b.width.toFixed(2); rec.th = +b.height.toFixed(2);
        break;
      }
    }
    out.push(rec);
    for (const c of el.children) walk(c, depth + 1);
  }
  walk(root, 0);
  const pre = document.createElement('pre'); pre.id = 'measure';
  pre.textContent = out.map(o => JSON.stringify(o)).join('\n');
  document.body.appendChild(pre);
});
</script>
"""


def main():
    name = sys.argv[1]
    out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "..", "boxes", name + ".jsonl")
    src = os.path.join(REPO, "docs", "design", "polish2-ref", name + ".html")
    tmp = os.path.join(tempfile.gettempdir(), "chipdaw-polish2-ref")
    os.makedirs(tmp, exist_ok=True)
    with open(src, encoding="utf-8") as f:
        page = f.read()
    page = page.replace("</body>", PROBE + "</body>") if "</body>" in page else page + PROBE
    dst = os.path.join(tmp, name + ".html")
    with open(dst, "w", encoding="utf-8", newline="\n") as f:
        f.write(page)
    url = "file:///" + dst.replace("\\", "/")
    dom = subprocess.run(
        [CHROME, "--headless=new", "--disable-gpu", "--window-size=1440,1000",
         "--virtual-time-budget=10000", "--dump-dom", url],
        capture_output=True, text=True, encoding="utf-8").stdout
    m = re.search(r'<pre id="measure">([\s\S]*?)</pre>', dom)
    if not m:
        sys.exit("no measurements in the dumped DOM")
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(html.unescape(m.group(1)) + "\n")
    print(out)


if __name__ == "__main__":
    main()
