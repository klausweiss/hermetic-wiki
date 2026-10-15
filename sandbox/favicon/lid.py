"""The jar lid's shape, shared by the favicon and the detailed lid.

Coordinates are in units of the 32x32 favicon grid; a picture N pixels wide
samples them at N / 32 pixels per unit. The lid's top faces left, the ridged
band runs off to the right, and the light comes from the top-left.
"""
import math

RX, RY, DEPTH = 10.4, 15.9, 8.5     # face ellipse radii, band depth
CX, CY = 16 - DEPTH / 2, 16.0       # face ellipse centre


def half_width(py):
    """Half the face's width at height py, or None above or below it."""
    v = (py - CY) / RY
    return RX * math.sqrt(1 - v * v) if abs(v) <= 1 else None


def inside(px, py):
    w = half_width(py)
    return w is not None and CX - w <= px <= CX + DEPTH + w


def arc(cx, a0, a1, steps=180):
    return [(cx + RX * math.cos(a0 + (a1 - a0) * i / steps),
             CY + RY * math.sin(a0 + (a1 - a0) * i / steps)) for i in range(steps + 1)]


# Silhouette: the face's left half joined to the back rim's right half.
OUTER = arc(CX, math.pi / 2, 3 * math.pi / 2) + arc(CX + DEPTH, -math.pi / 2, math.pi / 2)
OUTER.append(OUTER[0])
# Where the face meets the band: the face's right half.
FACE_RIM = arc(CX, -math.pi / 2, math.pi / 2)


def dist(px, py, poly):
    best = math.inf
    for (ax, ay), (bx, by) in zip(poly, poly[1:]):
        dx, dy = bx - ax, by - ay
        t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy or 1)))
        best = min(best, math.hypot(px - ax - t * dx, py - ay - t * dy))
    return best


# The H on the face, drawn into a trapezoid so it lies on the angled face:
# (x, top, bottom) of its left and right edges.
H_LEFT, H_RIGHT = (7.5, 10.0, 21.5), (15.5, 9.0, 23.0)
STEM, BAR = 0.29, (0.42, 0.58)       # in the letter's own 0..1 coordinates


def in_letter(px, py):
    (xl, tl, bl), (xr, tr, br) = H_LEFT, H_RIGHT
    s = (px - xl) / (xr - xl)
    if not 0 <= s <= 1:
        return False
    top, bot = tl + (tr - tl) * s, bl + (br - bl) * s
    t = (py - top) / (bot - top)
    if not 0 <= t <= 1:
        return False
    return s < STEM or s > 1 - STEM or BAR[0] <= t <= BAR[1]


def svg(grid, colours):
    """Pixel art as an SVG: one path per colour, one rectangle per run."""
    n = len(grid)
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" shape-rendering="crispEdges">']
    for c, fill in colours.items():
        d = []
        for y, row in enumerate(grid):
            x = 0
            while x < n:
                if row[x] == c:
                    s = x
                    while x < n and row[x] == c:
                        x += 1
                    d.append(f"M{s} {y}h{x-s}v1h-{x-s}z")
                else:
                    x += 1
        if d:
            out.append(f'<path fill="{fill}" d="{"".join(d)}"/>')
    out.append("</svg>")
    return "\n".join(out) + "\n"
