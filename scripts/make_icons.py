#!/usr/bin/env python3
"""Generate KeepAwake tray icon frames (cup_0.png .. cup_4.png).

Recreates the 5-frame coffee-cup concept from indicator.png as
monochrome macOS *template* images: pure black shapes on a transparent
background so the system can tint them for light/dark menu bars.

Drawn at high supersampling and downscaled with LANCZOS for clean
anti-aliasing. No external rasterizer needed (Pillow only).
"""
import math
import os
from PIL import Image, ImageDraw

# ---- logical geometry (0..100 square) ----------------------------------
S = 16                      # supersampling factor
N = 100 * S                 # supersampled canvas size (square)
def u(v): return int(round(v * S))   # logical -> supersampled px

STROKE = 5.0                # outline thickness (logical)
INK = (0, 0, 0, 255)        # template black

# Cup body trapezoid (slightly tapered, narrower at the bottom).
CX = 44                     # cup centre x (room for handle on the right)
TOP_Y, BOT_Y = 34, 82
TOP_HW, BOT_HW = 24, 20     # half widths at top / bottom

TL = (CX - TOP_HW, TOP_Y)
TR = (CX + TOP_HW, TOP_Y)
BR = (CX + BOT_HW, BOT_Y)
BL = (CX - BOT_HW, BOT_Y)


def lerp(a, b, t):
    return a + (b - a) * t


def cup_outline(draw):
    pts = [TL, TR, BR, BL]
    draw.line([u(x) for p in (pts + [pts[0]]) for x in p],
              fill=INK, width=u(STROKE), joint="curve")
    # round the corners a touch
    r = u(STROKE) // 2
    for x, y in pts:
        draw.ellipse([u(x) - r, u(y) - r, u(x) + r, u(y) + r], fill=INK)


def handle(draw):
    # Left-opening "C" hugging the right edge of the cup.
    cx, cy, rad = 70, 56, 17
    bbox = [u(cx - rad), u(cy - rad), u(cx + rad), u(cy + rad)]
    draw.arc(bbox, start=-78, end=78, fill=INK, width=u(STROKE))


def left_x(y):   # cup interior left edge x at height y
    t = (y - TOP_Y) / (BOT_Y - TOP_Y)
    return lerp(TL[0], BL[0], t) + STROKE * 0.7


def right_x(y):  # cup interior right edge x at height y
    t = (y - TOP_Y) / (BOT_Y - TOP_Y)
    return lerp(TR[0], BR[0], t) - STROKE * 0.7


def fill_liquid(base, fraction):
    """Diagonal-striped liquid filling the lower `fraction` of the cup."""
    if fraction <= 0:
        return
    inner_top = TOP_Y + STROKE * 0.8
    inner_bot = BOT_Y - STROKE * 0.8
    surface_y = lerp(inner_bot, inner_top, fraction)

    # interior region mask, below the liquid surface
    mask = Image.new("L", (N, N), 0)
    md = ImageDraw.Draw(mask)
    steps = 80
    poly = []
    ys = [lerp(surface_y, inner_bot, i / steps) for i in range(steps + 1)]
    for y in ys:
        poly.append((u(left_x(y)), u(y)))
    for y in reversed(ys):
        poly.append((u(right_x(y)), u(y)))
    md.polygon(poly, fill=255)

    # diagonal stripes layer
    stripes = Image.new("L", (N, N), 0)
    sd = ImageDraw.Draw(stripes)
    period = u(11)            # stripe pitch
    bar = u(6)                # stripe thickness
    slope = 0.45              # lower-left -> upper-right tilt
    span = N * 2
    off = -N
    while off < span:
        x0, y0 = off, N
        x1, y1 = off + int(slope * N), 0
        sd.line([x0, y0, x1, y1], fill=255, width=bar)
        off += period

    # keep stripes only inside the liquid region
    from PIL import ImageChops
    liquid = ImageChops.multiply(stripes, mask)
    ink = Image.new("RGBA", (N, N), INK)
    base.paste(ink, (0, 0), liquid)


def steam(draw):
    # three slightly slanted wavy plumes above the cup
    for k, bx in enumerate((CX - 12, CX - 1, CX + 10)):
        pts = []
        top, bottom = 9, 30
        n = 40
        for i in range(n + 1):
            t = i / n
            y = lerp(bottom, top, t)
            x = bx + 3.2 * math.sin(t * math.pi * 2.0 + k * 0.6) + t * 4.5
            pts.append((u(x), u(y)))
        flat = [c for p in pts for c in p]
        draw.line(flat, fill=INK, width=u(2.6), joint="curve")


# fraction = remaining-time fill level; steam on the warmer/fuller frames
FRAMES = [
    ("cup_0", 0.00, False),   # empty  -> inactive / expired
    ("cup_1", 0.12, False),   # low
    ("cup_2", 0.34, True),    # mid
    ("cup_3", 0.62, True),    # high
    ("cup_4", 0.90, True),    # full   -> freshly started
]

SIZES = [(32, ""), (64, "@2x")]   # cup_N.png (32) + cup_N@2x.png (64)

OUT = os.path.join(os.path.dirname(__file__), "..", "assets")
os.makedirs(OUT, exist_ok=True)

for name, frac, has_steam in FRAMES:
    img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    fill_liquid(img, frac)
    d = ImageDraw.Draw(img)
    cup_outline(d)
    handle(d)
    if has_steam:
        steam(d)
    for px, suffix in SIZES:
        out = img.resize((px, px), Image.LANCZOS)
        path = os.path.join(OUT, f"{name}{suffix}.png")
        out.save(path)
        print("wrote", os.path.relpath(path))
