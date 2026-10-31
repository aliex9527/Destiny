# -*- coding: utf-8 -*-
"""T1 数据柱的三种改法：换配色 + 换结构。

原版的问题：7 根柱高是随手排的，读起来像音乐播放器的均衡器，不像数据。
改法：
  · 前区 5 根柱按钟形频率排（和 T2 同一条曲线），后区 2 根明显更矮
  · 5 与 2 之间留一道更宽的缝，把「5+2」用间距说出来，不再依赖颜色
  · 加一条基线，柱子站在线上才像图表

  A1 深红底 + 白柱（后区 60% 白）/ 基线
  A2 深红底 + 白柱，均匀排列（只换色，保留原结构）
  A3 米白底 + 红柱 / 基线
"""
import os
import argparse
import numpy as np
from PIL import Image, ImageDraw, ImageFont

import icon_design as D
import icon_v7 as V

OUT = os.path.join(D.APP_DIR, "build", "icon_v9")

RED = (196, 23, 15)
RED_DEEP = (168, 14, 10)
WHITE = (255, 253, 250)
IVORY = (250, 246, 240)


def _rr(n, x0, y0, x1, y1, rad, color, alpha=255):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * x0, n * y0, n * x1, n * y1], radius=n * rad, fill=tuple(color) + (alpha,))
    return lay


def _finish(n, img, hairline):
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
        outline=hairline, width=max(1, int(n * 0.007)))
    lay.putalpha(Image.composite(lay.split()[3], Image.new("L", (n, n), 0), mask))
    return Image.alpha_composite(out, lay)


def _bar(n, x, h, w, base_y, color):
    """柱：圆头、平底。平底才能真的站在基线上，圆底看起来像悬空。"""
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay)
    r = n * w / 2.0
    d.rounded_rectangle([n * x, n * (base_y - h), n * (x + w), n * base_y],
                        radius=r, fill=tuple(color) + (255,))
    d.rectangle([n * x, n * (base_y - w / 2.0), n * (x + w), n * base_y],
                fill=tuple(color) + (255,))
    return lay


# 前区 5 根的相对高度：钟形，中间最高
FRONT = (0.330, 0.470, 0.545, 0.455, 0.345)
BACK = (0.215, 0.275)


def build(n, key):
    if key == "A3":
        bg, col, base_rule = IVORY, RED_DEEP, RED_DEEP
        hl = (0, 0, 0, 26)
    else:
        bg, col, base_rule = RED_DEEP, WHITE, WHITE
        hl = (255, 255, 255, 30)
    if key == "A2":
        base_rule = None

    img = V._flat(n, bg)
    w, gap, split = 0.052, 0.023, 0.082
    base_y = 0.752
    hs = list(FRONT) + list(BACK)
    total = 7 * w + 6 * gap + (split - gap)
    x = 0.5 - total / 2.0

    for i, h in enumerate(hs):
        img = Image.alpha_composite(img, _bar(n, x, h, w, base_y, col))
        x += w + gap
        if i == 4:
            x += split - gap

    if base_rule is not None:
        x0, x1 = 0.5 - total / 2.0 - 0.028, 0.5 + total / 2.0 + 0.028
        img = Image.alpha_composite(img, _rr(n, x0, base_y + 0.022, x1, base_y + 0.040,
                                             0.009, base_rule))
    return _finish(n, img, hl)


NAMES = {"A1": "红底钟形柱", "A2": "红底无基线", "A3": "米白钟形柱"}


def render(size, key):
    ss = 4 if size <= 256 else 2
    return build(size * ss, key).resize((size, size), Image.LANCZOS)


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
    keys = [k.strip().upper() for k in args.only.split(",") if k.strip()] or list(NAMES)
    sizes = [int(s) for s in args.sizes.split(",")]
    for k in keys:
        for sz in sizes:
            render(sz, k).save(os.path.join(OUT, "%s_%d.png" % (k, sz)))
        print(k, NAMES[k], "ok")
    sheet(keys).save(os.path.join(OUT, "sheet_v9.png"))


if __name__ == "__main__":
    main()
