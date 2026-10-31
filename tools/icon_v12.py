# -*- coding: utf-8 -*-
"""T1 数据柱：结构一个字不动（钟形 5 柱 + 2 矮柱 + 基线），只换颜色。

  C1 旧图标色  米白底 (255,253,251) + 5 红 (196,23,15) + 2 蓝 (26,106,238)
  C2 单色红    米白底 + 全部红柱，靠高度差区分前后区
  C3 红底白柱  深红底 (168,14,10) + 白柱
"""
import os
import argparse
from PIL import Image, ImageDraw, ImageFont

import icon_design as D
import icon_v7 as V
import icon_v9 as A

OUT = os.path.join(D.APP_DIR, "build", "icon_v12")

RED = (196, 23, 15)
RED_DEEP = (168, 14, 10)
BLUE = (26, 106, 238)
WHITE = (255, 253, 250)
BG_IVORY = (255, 253, 251)

# key: (名称, 底色, 前区柱色, 后区柱色, 基线色, 描边色)
PALETTES = {
    "C1": ("旧图标色", BG_IVORY, RED, BLUE, RED, (0, 0, 0, 28)),
    "C2": ("米白单色红", BG_IVORY, RED, RED, RED, (0, 0, 0, 28)),
    "C3": ("深红底白柱", RED_DEEP, WHITE, WHITE, WHITE, (255, 255, 255, 30)),
}

W, GAP, SPLIT = 0.052, 0.023, 0.082
BASE_Y = 0.752


def build(n, key):
    name, bg, c1, c2, rule, hl = PALETTES[key]
    img = V._flat(n, bg)
    hs = list(A.FRONT) + list(A.BACK)
    total = 7 * W + 6 * GAP + (SPLIT - GAP)
    x = 0.5 - total / 2.0
    for i, h in enumerate(hs):
        img = Image.alpha_composite(img, A._bar(n, x, h, W, BASE_Y, c2 if i >= 5 else c1))
        x += W + GAP
        if i == 4:
            x += SPLIT - GAP
    x0, x1 = 0.5 - total / 2.0 - 0.028, 0.5 + total / 2.0 + 0.028
    img = Image.alpha_composite(img, A._rr(n, x0, BASE_Y + 0.022, x1, BASE_Y + 0.040,
                                           0.009, rule))
    return A._finish(n, img, hl)


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
    sheet(keys).save(os.path.join(OUT, "sheet_v12.png"))


if __name__ == "__main__":
    main()
