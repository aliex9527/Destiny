# -*- coding: utf-8 -*-
"""T2 分布曲线 × 之前的几套配色。

结构不动（钟形曲线 + 曲线下淡填充），只换底色和曲线色：
  R1 深红底   (168,14,10) + 白曲线         —— 第六轮的深红底
  R2 正红底   (196,23,15) + 白曲线         —— 前几轮的品牌红
  R3 米白底   (250,246,240) + 深红曲线      —— 第三轮的 B 白玉
  R4 墨底红   (18,21,28) + 红曲线           —— 第七轮的深底，但点缀换回品牌红

用法：
    python tools/icon_v8.py
    python tools/icon_v8.py --only R1,R3 --sizes 512,48
"""
import os
import argparse
import numpy as np
from PIL import Image, ImageDraw, ImageFont

import icon_design as D
import icon_v7 as V

OUT = os.path.join(D.APP_DIR, "build", "icon_v8")

RED = (196, 23, 15)
RED_DEEP = (168, 14, 10)
RED_BRIGHT = (255, 82, 66)
WHITE = (255, 253, 250)
IVORY = (250, 246, 240)
INK = V.INK

PALETTES = {
    "R1": ("深红底", RED_DEEP, WHITE, 34, (255, 255, 255, 30)),
    "R2": ("正红底", RED, WHITE, 34, (255, 255, 255, 30)),
    "R3": ("米白底", IVORY, RED_DEEP, 30, (0, 0, 0, 26)),
    "R4": ("墨底红", INK, RED_BRIGHT, 40, (255, 255, 255, 34)),
}


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


def build(n, key):
    name, bg, fg, alpha, hl = PALETTES[key]
    img = V._flat(n, bg)

    xs = np.linspace(0.098, 0.902, 200)
    mu, sig, amp, base_y = 0.5, 0.148, 0.410, 0.700
    ys = base_y - amp * np.exp(-((xs - mu) ** 2) / (2 * sig ** 2))

    poly = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    pts = [(n * xs[0], n * base_y)] + [(n * x, n * y) for x, y in zip(xs, ys)] + \
          [(n * xs[-1], n * base_y)]
    ImageDraw.Draw(poly).polygon(pts, fill=tuple(fg) + (alpha,))
    img = Image.alpha_composite(img, poly)
    img = Image.alpha_composite(img, V._stroke(n, list(zip(xs.tolist(), ys.tolist())), 0.042, fg))
    return _finish(n, img, hl)


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
        d.text((pad - 4, y + max(sizes) / 2 - 12), "%s %s" % (k, PALETTES[k][0]),
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
    keys = [k.strip().upper() for k in args.only.split(",") if k.strip()] or list(PALETTES)
    sizes = [int(s) for s in args.sizes.split(",")]
    for k in keys:
        for sz in sizes:
            render(sz, k).save(os.path.join(OUT, "%s_%d.png" % (k, sz)))
        print(k, PALETTES[k][0], "ok")
    sheet(keys).save(os.path.join(OUT, "sheet_v8.png"))


if __name__ == "__main__":
    main()
