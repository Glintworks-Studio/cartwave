from .Image import Image


def difference(a, b):
    return Image(a.width, a.height, bytes(abs(x - y) for x, y in zip(a._d, b._d)))
