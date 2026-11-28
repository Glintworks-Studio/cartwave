"""device-mockup-match.png: the board's device (left) and ours (right), studio above, locked below.
usage: python compose_match_pb.py <ours studio.png> <ours locked.png> <out.png>"""
import sys
from PIL import Image, ImageDraw

board = Image.open("docs/design/meeting-2026-11-26/workflow-chip-synth@1x.png").convert("RGB")
studio, locked = Image.open(sys.argv[1]).convert("RGB"), Image.open(sys.argv[2]).convert("RGB")
W, H, P = 880 + 24, 332 + 24, 12
out = Image.new("RGB", (2 * W + P, 2 * H + 40), (8, 8, 8))
d = ImageDraw.Draw(out)
for row, (img, bo) in enumerate(((studio, (224, 532)), (locked, (224, 960)))):
    ct = img.height - 920
    cl = (img.width - 1440) // 2
    oo = (cl + 224, ct + 552)
    out.paste(board.crop((bo[0] - 12, bo[1] - 12, bo[0] - 12 + W, bo[1] - 12 + H)), (0, 20 + row * (H + 20)))
    out.paste(img.crop((oo[0] - 12, oo[1] - 12, oo[0] - 12 + W, oo[1] - 12 + H)), (W + P, 20 + row * (H + 20)))
    d.text((4, 4 + row * (H + 20)), "board: " + ("studio" if row == 0 else "locked"), fill=(200, 200, 200))
    d.text((W + P + 4, 4 + row * (H + 20)), "Cartwave: " + ("studio" if row == 0 else "locked"), fill=(200, 200, 200))
out.save(sys.argv[3])
