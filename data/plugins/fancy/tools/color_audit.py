"""Audit the badge palette and measure colors in PNGs, standard library only.

  python3 tools/color_audit.py palette "DONE:#13651A,AUTO:#4130DF>#9B1FE8>#C2185B"
  python3 tools/color_audit.py png badge.png screenshot.png
"""
import itertools
import math
import struct
import sys
import zlib
from collections import Counter

CVD = {  # Machado et al. 2009, severity 1, linear RGB
    "deuteranopia": [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
    "protanopia": [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
    "tritanopia": [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
}
SRGB_D50 = [[0.4360747, 0.3850649, 0.1430804], [0.2225045, 0.7168786, 0.0606169], [0.0139322, 0.0971045, 0.7141733]]


def rgb(hex_):
    return [int(hex_[i:i + 2], 16) / 255 for i in (1, 3, 5)]


def lin(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def enc(c):
    c = min(max(c, 0), 1)
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def hex_of(channels):
    return "#" + "".join(f"{round(c * 255):02X}" for c in channels)


def luminance(hex_):
    r, g, b = map(lin, rgb(hex_))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b):
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def lab(hex_, deficiency=None):
    v = list(map(lin, rgb(hex_)))
    if deficiency:
        m = CVD[deficiency]
        v = [min(max(sum(m[i][j] * v[j] for j in range(3)), 0), 1) for i in range(3)]
    r, g, b = v
    x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883
    f = lambda t: t ** (1 / 3) if t > 0.008856 else 7.787 * t + 16 / 116
    return 116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))


def gradient(stops, cells):
    if len(stops) < 2:
        return [stops[0]] * cells
    out = []
    for i in range(cells):
        t = i / (cells - 1) * (len(stops) - 1)
        k = min(int(t), len(stops) - 2)
        a, b = rgb(stops[k]), rgb(stops[k + 1])
        out.append(hex_of([x + (y - x) * (t - k) for x, y in zip(a, b)]))
    return out


def audit_palette(spec, terminal_bg="#1E1E1E"):
    tags = dict(pair.split(":", 1) for pair in spec.split(","))
    solid = {}
    print(f"{'tag':6} {'texto':7} {'contraste':>9} {'nível':6} {'vs fundo':>8}")
    for name, color in tags.items():
        cells = gradient(color.split(">"), len(name) + 2)
        white = min(contrast(c, "#FFFFFF") for c in cells)
        black = min(contrast(c, "#000000") for c in cells)
        text, ratio = ("branco", white) if white >= black else ("preto", black)
        level = "AAA" if ratio >= 7 else "AA" if ratio >= 4.5 else "FALHA"
        edge = min(contrast(c, terminal_bg) for c in cells)
        print(f"{name:6} {text:7} {ratio:9.2f} {level:6} {edge:8.2f}")
        solid[name] = cells[len(cells) // 2]
    print(f"\nmenor distância perceptual (ΔE76) entre tags; abaixo de ~2.3 o olho não separa:")
    for mode in (None, *CVD):
        d, a, b = min((math.dist(lab(solid[a], mode), lab(solid[b], mode)), a, b)
                      for a, b in itertools.combinations(solid, 2))
        print(f"  {mode or 'visão normal':13} {d:5.1f}  {a} x {b}")


def read_png(path):
    data = open(path, "rb").read()
    i, idat, icc, srgb = 8, b"", None, False
    while i < len(data):
        n, = struct.unpack(">I", data[i:i + 4])
        kind, body = data[i + 4:i + 8], data[i + 8:i + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype = struct.unpack(">IIBB", body[:10])
        elif kind == b"IDAT":
            idat += body
        elif kind == b"iCCP":
            icc = zlib.decompress(body[body.index(b"\0") + 2:])
        elif kind == b"sRGB":
            srgb = True
        i += 12 + n
    if depth != 8 or ctype not in (2, 6):
        raise SystemExit(f"{path}: só PNG de 8 bits RGB ou RGBA")
    bpp = 3 if ctype == 2 else 4
    raw, stride, rows, prev, p = zlib.decompress(idat), w * bpp, [], bytearray(w * bpp), 0
    for _ in range(h):
        f, line = raw[p], bytearray(raw[p + 1:p + 1 + stride])
        p += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            up, c = prev[x], prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + up) & 255
            elif f == 3:
                line[x] = (line[x] + (a + up) // 2) & 255
            elif f == 4:
                q = a + up - c
                pa, pb, pc = abs(q - a), abs(q - up), abs(q - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else up if pb <= pc else c)) & 255
        rows.append(line)
        prev = line
    pixels = [tuple(r[x * bpp:x * bpp + 3]) for r in rows for x in range(w)
              if bpp == 3 or r[x * bpp + 3] == 255]
    return pixels, icc, srgb


def icc_to_srgb(icc):
    """Converter from a matrix/TRC display profile (the kind macOS screenshots embed) to sRGB."""
    count, = struct.unpack(">I", icc[128:132])
    tags = {}
    for k in range(count):
        sig, off, size = struct.unpack(">4sII", icc[132 + 12 * k:144 + 12 * k])
        tags[sig] = icc[off:off + size]
    xyz = lambda s: [v / 65536 for v in struct.unpack(">iii", tags[s][8:20])]

    def trc(s):
        d = tags[s]
        if d[:4] == b"curv":
            n, = struct.unpack(">I", d[8:12])
            if n == 1:
                g = struct.unpack(">H", d[12:14])[0] / 256
                return lambda v: v ** g
            table = [x / 65535 for x in struct.unpack(f">{n}H", d[12:12 + 2 * n])]
            return lambda v: table[min(n - 1, round(v * (n - 1)))]
        kind, = struct.unpack(">H", d[8:10])
        p = [x / 65536 for x in struct.unpack(f">{(len(d) - 12) // 4}i", d[12:])] + [1, 0, 0, 0]
        g, a, b, c, e = p[0], p[1], p[2], p[3], p[4]
        if kind == 0:
            return lambda v: v ** g
        return lambda v: (a * v + b) ** g if v >= e else c * v

    m = list(zip(xyz(b"rXYZ"), xyz(b"gXYZ"), xyz(b"bXYZ")))
    curves = [trc(b"rTRC"), trc(b"gTRC"), trc(b"bTRC")]
    s = SRGB_D50
    det = (s[0][0] * (s[1][1] * s[2][2] - s[1][2] * s[2][1]) - s[0][1] * (s[1][0] * s[2][2] - s[1][2] * s[2][0])
           + s[0][2] * (s[1][0] * s[2][1] - s[1][1] * s[2][0]))
    inv = [[(s[(j + 1) % 3][(i + 1) % 3] * s[(j + 2) % 3][(i + 2) % 3]
             - s[(j + 1) % 3][(i + 2) % 3] * s[(j + 2) % 3][(i + 1) % 3]) / det for j in range(3)] for i in range(3)]

    def convert(px):
        v = [curves[j](px[j] / 255) for j in range(3)]
        x = [sum(m[r][c] * v[c] for c in range(3)) for r in range(3)]
        return hex_of([enc(sum(inv[r][c] * x[c] for c in range(3))) for r in range(3)])
    return convert


def audit_png(paths, top=1):
    for path in paths:
        pixels, icc, srgb = read_png(path)
        convert = icc_to_srgb(icc) if icc else (lambda px: "#%02X%02X%02X" % px)
        profile = "ICC embutido, convertido para sRGB" if icc else "sRGB" if srgb else "sem perfil, lido como sRGB"
        colors = ", ".join(f"{convert(c)} ({n} px)" for c, n in Counter(pixels).most_common(top))
        print(f"{path}: {colors}  [{profile}]")


if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "palette":
        audit_palette(sys.argv[2], *(sys.argv[3:4]))
    elif len(sys.argv) >= 3 and sys.argv[1] == "png":
        audit_png(sys.argv[2:])
    else:
        print(__doc__)
