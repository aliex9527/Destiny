# -*- coding: utf-8 -*-
"""复刻你截图里那一版 T1 的几何（深底 + 7 根圆头柱 + 5 白 2 蓝），只换颜色。

几何参数与 `icon_v7.t1` 完全一致，未做任何改动：
    柱宽 0.062、间距 0.024、基线 y=0.672、
    高度 (0.170, 0.288, 0.230, 0.372, 0.276, 0.208, 0.330)、圆头圆底。

四套配色：
    D1 旧图标色   米白底 + 红 + 蓝
    D2 深红底     深红底 + 白 + 白（去掉蓝）
    D3 深红底蓝   深红底 + 白 + 亮蓝
    D4 墨底红     原深底 + 红 + 亮蓝
"""
import os
import argparse
from PIL import Image, ImageDraw, ImageFont

import icon_design as D
import icon_v7 as V

OUT = os.path.join(D.APP_DIR, "build", "icon_v13")

RED = (196, 23, 15)
RED_DEEP = (168, 14, 10)
RED_BRIGHT = (255, 82, 66)
BLUE = (26, 106, 238)
BLUE_BRIGHT = (90, 160, 255)
WHITE = (255, 253, 250)
BG_IVORY = (255, 253, 251)
INK = V.INK

W, GAP, BASE = 0.062, 0.024, 0.672
HS = (0.170, 0.288, 0.230, 0.372, 0.276, 0.208, 0.330)

# key: (名称, 底色, 前五根色, 后两根色, 描边色)
PALETTES = {
    "D1": ("旧图标色", BG_IVORY, RED, BLUE, (0, 0, 0, 28)),
    "D2": ("深红底全白", RED_DEEP, WHITE, WHITE, (255, 255, 255, 30)),
    "D3": ("深红底蓝点", RED_DEEP, WHITE, BLUE_BRIGHT, (255, 255, 255, 30)),
    "D4": ("墨底红柱", INK, RED_BRIGHT, BLUE_BRIGHT, (255, 255, 255, 34)),
}


def build(n, key):
    name, bg, c1, c2, hl = PALETTES[key]
    img = V._flat(n, bg)
    left = 0.5 - (7 * W + 6 * GAP) / 2.0
    for i, h in enumerate(HS):
        x = left + i * (W + GAP)
        img = Image.alpha_composite(
            img, V._rr(n, x, BASE - h, x + W, BASE, W / 2.0, c2 if i >= 5 else c1))
    return A_finish(n, img, hl)


def A_finish(n, img, hl):
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
        outline=hl, width=max(1, int(n * 0.007)))
    lay.putalpha(Image.composite(lay.split()[3], Image.new("L", (n, n), 0), mask))
    return Image.alpha_composite(out, lay)


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
    sheet(keys).save(os.path.join(OUT, "sheet_v13.png"))


if __name__ == "__main__":
    main()
