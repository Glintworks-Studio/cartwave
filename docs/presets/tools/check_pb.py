"""Pixel checks for the PB scenarios (at_pb.ps1).

usage: check_pb.py dim <locked.png> <studio.png>
  The device on a locked track is dimmed (displays and knobs at about 40 %) and its labels
  stay readable (>= 4.5 : 1): the waveform display's brightest pixel value on the locked track
  is 25-55 % of the studio track's, and the locked Duty label's ink against the section behind
  it reaches 4.5 : 1. Screenshots are the window's frame (shot.ps1): client px + (1, 52) at 100 %.
"""
import sys
from PIL import Image

OX, OY = 1, 52


def lum(c):
    def ch(v):
        v /= 255.0
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    return 0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])


def region(img, x0, y0, x1, y1):
    return [img.getpixel((x, y))[:3] for y in range(y0 + OY, y1 + OY) for x in range(x0 + OX, x1 + OX)]


def dim(locked, studio):
    a, b = Image.open(locked).convert("RGB"), Image.open(studio).convert("RGB")
    # The OSCILLATOR display: device (224, 552) + (24, 90, 232, 96).
    # Brightness as drawn (the sRGB value of the brightest pixel), the way "40 %" is meant: a
    # layer at 40 % alpha; in linear light that is about 0.4^2.2 (the first run's 0.21).
    la = max(max(c) for c in region(a, 250, 644, 478, 736))
    lb = max(max(c) for c in region(b, 250, 644, 478, 736))
    ratio = la / lb if lb else 0
    # The locked Duty label: centred at x 364, its line 796..808 (client: the knob centre 770).
    px = region(a, 340, 794, 388, 810)
    ink = max(lum(c) for c in px)
    bg = min(lum(c) for c in px)
    contrast = (ink + 0.05) / (bg + 0.05)
    ok = 0.25 <= ratio <= 0.55 and contrast >= 4.5
    print(f"{'ok' if ok else 'bad'} display {ratio:.2f} of the studio one; locked label contrast {contrast:.2f}")


if __name__ == "__main__":
    if sys.argv[1] == "dim":
        dim(sys.argv[2], sys.argv[3])
