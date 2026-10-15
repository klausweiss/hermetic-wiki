"""The jar lid in more detail: more tones, dithering, ridges with lit tops.

Usage: python3 detailed.py OUT.svg [--size N] [--pad P --background COLOUR]

The lid is drawn on an N-pixel grid (128 by default); --pad surrounds it with
P pixels of --background, for icons that must not be transparent.
"""
import argparse

from lid import CX, CY, DEPTH, FACE_RIM, OUTER, RX, RY, dist, half_width, in_letter, inside, svg

OUTLINE, SEAM = 1.5, 1.25             # stroke widths, in favicon units

COLOURS = {
    "K": "#10131c",   # ink
    "E": "#1f4597",   # deepest blue
    "D": "#2a5cc8",
    "M": "#3a73e8",
    "B": "#4888ff",
    "N": "#6399ff",
    "L": "#7eacff",
    "H": "#b4d0ff",   # soft highlight
    "W": "#ffffff",
}

BAYER = [[0, 2], [3, 1]]


def face(px, py, x, y, edge):
    w = half_width(py)
    u = (px - CX) / w
    v = (py - CY) / RY
    # glare: a bright arc along the upper-left rim with a soft halo, plus a glint
    if u < -0.1 and -0.8 < v < 0.3:
        if OUTLINE + 0.9 <= edge < OUTLINE + 1.9:
            return "W"
        if OUTLINE + 0.6 <= edge < OUTLINE + 2.3:
            return "H"
    gx, gy = px - (CX - 2.2), (py - (CY - 10.2)) / 0.9
    if gx * gx + gy * gy < 1:
        return "W"
    # engraved H: the cut runs rightwards into the lid
    d = 0.6
    if in_letter(px, py):
        if not in_letter(px - d, py):
            return "E"                  # left wall, in shadow
        if not in_letter(px - 2 * d, py) and (x + y) % 2:
            return "E"                  # dithered falloff of that shadow
        return "D"
    if in_letter(px - 0.3, py):
        return "N"                      # the lit lip on the cut's right edge

    # shading: crescents along the lower-right rim, outside copies of the face
    # shifted up-left, dithered where tones meet
    def shade(k):
        lx, ly = (px - CX + 2.2 * k) / RX, (py - CY + 2.5 * k) / RY
        return lx * lx + ly * ly

    r1, r2 = shade(1.0), shade(0.45)
    if r1 > 1:
        return "B"
    if r2 > 1:
        return "N" if r1 < 0.96 or BAYER[y % 2][x % 2] >= 2 else "B"
    if r2 > 0.94:
        return "L" if BAYER[y % 2][x % 2] >= 2 else "N"
    return "L"


def band(px, py, scale):
    v = (py - CY) / RY
    phase = (py * scale) % 6           # a ridge every 6 pixels
    if v < -0.55:
        tones = "DHLLLL"
    elif v < -0.1:
        tones = "DLBBBB"
    elif v < 0.4:
        tones = "ENBBMM"
    else:
        tones = "KMDDDD" if v > 0.75 else "EMDDDD"
    c = tones[int(phase)]
    # the far rim of the band falls into shade
    if dist(px, py, OUTER) < OUTLINE + 0.6 and px > CX + DEPTH and c not in "KE":
        c = {"H": "L", "L": "B", "B": "M", "N": "B", "M": "D", "D": "E"}[c]
    return c


def pixel(x, y, scale):
    px, py = (x + 0.5) / scale, (y + 0.5) / scale
    if not inside(px, py):
        return "."
    edge = dist(px, py, OUTER)
    if edge < OUTLINE:
        return "K"
    w = half_width(py)
    if px - CX <= w:
        if dist(px, py, FACE_RIM) < SEAM / 2 + 0.5 / scale:
            return "K"
        return face(px, py, x, y, edge)
    if px - (CX + w) < SEAM / 2 + 0.6:
        return "E"                      # shadow cast by the face's rim
    return band(px, py, scale)


def grid(size):
    scale = size / 32
    return ["".join(pixel(x, y, scale) for x in range(size)) for y in range(size)]


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("out")
    p.add_argument("--size", type=int, default=128)
    p.add_argument("--pad", type=int, default=0)
    p.add_argument("--background")
    a = p.parse_args()
    rows = grid(a.size)
    colours = dict(COLOURS)
    if a.pad:
        side = "." * (a.size + 2 * a.pad)
        rows = [side] * a.pad + ["." * a.pad + r + "." * a.pad for r in rows] + [side] * a.pad
    if a.background:
        rows = [r.replace(".", "_") for r in rows]
        colours = {"_": a.background, **colours}
    with open(a.out, "w") as f:
        f.write(svg(rows, colours))
