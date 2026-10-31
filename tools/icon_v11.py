# -*- coding: utf-8 -*-
"""T1 与旧图标（米白底 + 红蓝球）对齐：柱不再用实心长方条，改用「球」的形态。

旧图标是近白微暖底 + 5 红 2 蓝球围环，所以这里：
  · 底色回到 (255,253,251)，红 (196,23,15)、蓝 (26,106,238) 沿用
  · 柱改为球：B1 棒棒糖（细杆 + 顶球）、B2 点阵柱（一列小球堆叠）、B3 点阵无基线
  · 5 红 2 蓝的前后区关系，靠颜色继续表达

用法：
    python tools/icon_v11.py
    python tools/icon_v11.py --only B1,B2 --sizes 512,48
"""
import os
import argparse
from PIL import Image, ImageDraw, ImageFont

import icon_design as D
import icon_v7 as V

OUT = os.path.join(D.APP_DIR, "build", "icon_v11")

BG = (255, 253, 251)
RED = (196, 23, 15)
BLUE = (26, 106, 238)

FRONT = (0.330, 0.470, 0.545, 0.455, 0.345)
BACK = (0.215, 0.275)


def _finish(n, img):
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
        outline=(0, 0, 0, 28), width=max(1, int(n * 0.007)))
    lay.putalpha(Image.composite(lay.split()[3], Image.new("L", (n, n), 0), mask))
    return Image.alpha_composite(out, lay)


def _stem(n, x, y0, y1, w, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * (x - w / 2.0), n * y0, n * (x + w / 2.0), n * y1],
        radius=n * w / 2.0, fill=tuple(color) + (255,))
    return lay


def b1(n):
    """棒棒糖：细杆 + 顶球。柱的高度还是数据，但每根柱头上都顶着一颗球。"""
    img = V._flat(n, BG)
    base_y = 0.768
    w, cg, split = 0.030, 0.076, 0.032
    span = 6 * cg + split          # 首个球心到末个球心的距离
    x = 0.5 - span / 2.0
    hs = list(FRONT) + list(BACK)
    for i, h in enumerate(hs):
        col = BLUE if i >= 5 else RED
        cy = base_y - h
        r = 0.050 if i < 5 else 0.046
        img = Image.alpha_composite(img, _stem(n, x, cy, base_y, w, col))
        img = Image.alpha_composite(img, V._disc(n, x, cy, r, col))
        x += cg
        if i == 4:
            x += split
    x0, x1 = 0.5 - span / 2.0 - 0.014, 0.5 + span / 2.0 + 0.014
    img = Image.alpha_composite(img, A_rr(n, x0, base_y + 0.020, x1, base_y + 0.034,
                                           0.007, RED, 205))
    return _finish(n, img)


def A_rr(n, x0, y0, x1, y1, rad, color, alpha=255):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * x0, n * y0, n * x1, n * y1], radius=n * rad, fill=tuple(color) + (alpha,))
    return lay


def _dots(n, img, x, count, d, gap, col, base_y):
    """一列圆点从基线往上排。"""
    for k in range(count):
        cy = base_y - k * (d + gap) - d / 2.0
        img = Image.alpha_composite(img, V._disc(n, x, cy, d / 2.0, col))
    return img


def b2(n):
    """点阵柱：每列由小球堆成，球数就是高度。5 红列 + 2 蓝列。"""
    img = V._flat(n, BG)
    base_y = 0.758
    d, gap, cg, split = 0.068, 0.022, 0.088, 0.036
    total = 7 * cg + split
    x = 0.5 - total / 2.0 + cg / 2.0
    counts = (3, 4, 5, 4, 3) + (2, 2)
    for i, c in enumerate(counts):
        col = BLUE if i >= 5 else RED
        img = _dots(n, img, x, c, d, gap, col, base_y)
        x += cg
        if i == 4:
            x += split
    x0, x1 = 0.5 - total / 2.0 - 0.010, 0.5 + total / 2.0 + 0.010
    img = Image.alpha_composite(img, A_rr(n, x0, base_y + 0.020, x1, base_y + 0.036, 0.008, RED))
    return _finish(n, img)


def b3(n):
    """点阵柱，去掉基线：纯球阵列，最贴近旧图标的构图。"""
    img = V._flat(n, BG)
    base_y = 0.740
    d, gap, cg, split = 0.072, 0.024, 0.092, 0.040
    total = 7 * cg + split
    x = 0.5 - total / 2.0 + cg / 2.0
    counts = (3, 4, 5, 4, 3) + (2, 2)
    for i, c in enumerate(counts):
        col = BLUE if i >= 5 else RED
        img = _dots(n, img, x, c, d, gap, col, base_y)
        x += cg
        if i == 4:
            x += split
    return _finish(n, img)


FNS = {"B1": b1, "B2": b2, "B3": b3}
NAMES = {"B1": "棒棒糖", "B2": "点阵柱", "B3": "点阵无基线"}


def render(size, key):
    ss = 4 if size <= 256 else 2
    return FNS[key](size * ss).resize((size, size), Image.LANCZOS)


def sheet(keys, sizes=(256, 96, 48)):
    pad, gap, label = 28, 24, 210
    w = pad * 2 + label + sum(sizes) + gap * (len(sizes) - 1)
    h = pad * 2 + len(keys) * (max(sizes) + gap)
    img = Image.new("RGBA", (int(w), int(h)), (232, 234, 238, 255))
    d = ImageDraw.Draw(img)
    f = ImageFont.truetype(D.FONT_BOLD, 20)
    for r, k in enumerate(keys):
        y = pad + r * (max(sizes) + gap)
        d.text((pad - 4, y + max(sizes) / 2 - 12), "%s %s" % (k, NAMES[k]),
               font=f, fill=(52, 56, 64, 255))
        x = pad + label
        for sz in sizes:
            img.alpha_composite(render(sz, k), (int(x), int(y + (max(sizes) - sz) / 2)))
            x += sz + gap
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sizes", default="512,192,96,48")
    ap.add_argument("--only", default="")
    args = ap.parse_args()
    os.makedirs(OUT, exist_ok=True)
    keys = [k.strip().upper() for k in args.only.split(",") if k.strip()] or list(FNS)
    sizes = [int(s) for s in args.sizes.split(",")]
    for k in keys:
        for sz in sizes:
            render(sz, k).save(os.path.join(OUT, "%s_%d.png" % (k, sz)))
        print(k, NAMES[k], "ok")
    sheet(keys).save(os.path.join(OUT, "sheet_v11.png"))


if __name__ == "__main__":
    main()
