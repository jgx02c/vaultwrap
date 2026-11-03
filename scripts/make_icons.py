#!/usr/bin/env python3
"""Render the app icon (1024px) and menu-bar template icon without
third-party libraries. Run: python3 scripts/make_icons.py <out dir>"""
import math, struct, sys, zlib

def png(path, w, h, px):
    raw = b"".join(b"\x00" + bytes(px[y * w * 4:(y + 1) * w * 4]) for y in range(h))
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))

def sd_round_rect(x, y, cx, cy, hw, hh, r):
    qx, qy = abs(x - cx) - hw + r, abs(y - cy) - hh + r
    return math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - r

def sd_ring_top(x, y, cx, cy, r, t, bottom):
    # Upper arc of a ring, extended straight down to `bottom` (the shackle).
    if y > cy:
        d = abs(abs(x - cx) - r) - t / 2
        return max(d, y - bottom)
    return abs(math.hypot(x - cx, y - cy) - r) - t / 2

def cov(d, aa=1.0):
    return max(0.0, min(1.0, 0.5 - d / aa))

def lock_shapes(s):
    """Signed distances (shackle, body, keyhole) in a unit box of size s."""
    def f(x, y):
        shackle = sd_ring_top(x, y, 0.5 * s, 0.40 * s, 0.135 * s, 0.065 * s, 0.52 * s)
        body = sd_round_rect(x, y, 0.5 * s, 0.63 * s, 0.215 * s, 0.17 * s, 0.05 * s)
        hole = min(math.hypot(x - 0.5 * s, y - 0.60 * s) - 0.042 * s,
                   sd_round_rect(x, y, 0.5 * s, 0.67 * s, 0.018 * s, 0.06 * s, 0.012 * s))
        return shackle, body, hole
    return f

def app_icon(path, S=1024):
    px = bytearray(S * S * 4)
    f = lock_shapes(S * 0.80)
    off = S * 0.10
    for y in range(S):
        for x in range(S):
            # macOS-style squircle-ish rounded square with margin
            bg = cov(sd_round_rect(x + .5, y + .5, S / 2, S / 2, S * 0.41, S * 0.41, S * 0.185))
            if bg == 0:
                continue
            t = y / S
            r, g, b = 30 + (79 - 30) * t, 27 + (70 - 27) * t, 75 + (229 - 75) * t  # indigo gradient
            sh, body, hole = f(x + .5 - off, y + .5 - off)
            fg = max(cov(sh), cov(body) * (1 - cov(hole)))
            r, g, b = r + (248 - r) * fg, g + (250 - g) * fg, b + (252 - b) * fg
            i = (y * S + x) * 4
            px[i:i + 4] = bytes((int(r), int(g), int(b), int(255 * bg)))
    png(path, S, S, px)

def tray_icon(path, S):
    px = bytearray(S * S * 4)
    f = lock_shapes(S)
    aa = S / 44
    for y in range(S):
        for x in range(S):
            sh, body, hole = f(x + .5, y + .5)
            a = max(cov(sh, aa), cov(body, aa) * (1 - cov(hole, aa)))
            i = (y * S + x) * 4
            px[i:i + 4] = bytes((0, 0, 0, int(255 * a)))
    png(path, S, S, px)

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "."
    app_icon(f"{out}/app-icon.png")
    tray_icon(f"{out}/tray.png", 22)
    tray_icon(f"{out}/tray@2x.png", 44)
    print("icons written to", out)
