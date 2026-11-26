class Draw:
    def __init__(self, im):
        self.im = im

    def rectangle(self, box, outline=None, fill=None, width=1):
        x0, y0, x1, y1 = (int(v) for v in box)
        px = self.im.load()
        for y in range(max(0, y0), min(self.im.height, y1 + 1)):
            for x in range(max(0, x0), min(self.im.width, x1 + 1)):
                edge = x in (x0, x1) or y in (y0, y1)
                if fill is not None and not edge:
                    px[x, y] = fill
                elif outline is not None and edge:
                    px[x, y] = outline
