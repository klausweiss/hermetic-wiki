"""The favicon: the jar lid on a 32x32 grid, with few colours so it reads at 16px.

Usage: python3 favicon.py OUT.svg
"""
import sys

from lid import CX, CY, FACE_RIM, OUTER, RX, RY, dist, half_width, in_letter, inside, svg

N = 32
OUTLINE, SEAM = 2.0, 2.0            # stroke widths in pixels

COLOURS = {"K": "#10131c", "E": "#1f4597", "D": "#2a5cc8", "B": "#4888ff",
           "L": "#7eacff", "W": "#ffffff"}


def pixel(x, y):
    px, py = x + 0.5, y + 0.5
    if not inside(px, py):
        return "."
    edge = dist(px, py, OUTER)
    if edge < OUTLINE:
        return "K"
    w = half_width(py)
    u = (px - CX) / w                 # -1..1 across the face
    v = (py - CY) / RY                # -1..1 top to bottom
    if u <= 1:                        # face
        if dist(px, py, FACE_RIM) < SEAM / 2 + 0.5:
            return "K"
        if OUTLINE + 1 <= edge < OUTLINE + 2.6 and u < -0.15 and -0.75 < v < 0.25:
            return "W"                # glare arc along the upper-left rim
        # shade: a crescent along the lower-right rim, outside a copy of the
        # face shifted up-left
        lx, ly = (px - CX + 2.2) / RX, (py - CY + 2.5) / RY
        if lx * lx + ly * ly > 1:
            return "B"
        return "L"
    # band: horizontal ridges, darker towards the bottom
    groove = y % 3 == 0
    if v < -0.5:
        return "D" if groove else "L"
    if v < 0.35:
        return "D" if groove else "B"
    return "E" if groove else "D"


def grid():
    rows = []
    for y in range(N):
        row = [pixel(x, y) for x in range(N)]
        for x in range(N):
            if in_letter(x + 0.5, y + 0.5):
                # engraved: the cut runs into the lid (rightwards), so its left
                # wall shows, facing away from the light
                row[x] = "D" if in_letter(x - 0.5, y + 0.5) else "E"
        rows.append("".join(row))
    return rows


if __name__ == "__main__":
    with open(sys.argv[1], "w") as f:
        f.write(svg(grid(), COLOURS))
