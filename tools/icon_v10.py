# -*- coding: utf-8 -*-
"""T1（钟形柱 + 基线）换配色。

结构完全不动，只换底色 / 柱色 / 基线色：
  P1 深红底   (168,14,10)  + 白柱           —— 现在这版
  P2 正红底   (196,23,15)  + 白柱           —— 更接近原启动图标
  P3 米白底   (250,246,240)+ 深红柱          —— 最通透
  P4 墨底红   (18,21,28)   + 亮红柱 (255,82,66) —— 科技感

用法：
    python tools/icon_v10.py
    python tools/icon_v10.py --only P1,P4 --sizes 512
"""
import os
import argparse
from PIL import Image, ImageFont, ImageDraw

import icon_design as D
import icon_v7 as V
import icon_v9 as A

OUT = os.path.join(D.APP_DIR, "build", "icon_v10")

PALETTES = {
    "P1": ("深红底", (168, 14, 10), (255, 253, 250), (255, 255, 255, 30)),
    "P2": ("正红底", (196, 23, 15), (255, 253, 250), (255, 255, 255, 30)),
    "P3": ("米白底", (250, 246, 240), (168, 14, 10), (0, 0, 0, 26)),
    "P4": ("墨底红柱", (18, 21, 28), (255, 82, 66), (255, 255, 255, 34)),
}

W, GAP, SPLIT = 0.052, 0.023, 0.082
BASE_Y = 0.752
FRONT = A.FRONT
BACK = A.BACK


def build(n, key):
    name, bg, col, hl = PALETTES[key]
    img = V._flat(n, bg)
    hs = list(FRONT) + list(BACK)
    total = 7 * W + 6 * GAP + (SPLIT - GAP)
    x = 0.5 - total / 2.0
    for i, h in enumerate(hs):
        img = Image.alpha_composite(img, A._bar(n, x, h, W, BASE_Y, col))
        x += W + GAP
        if i == 4:
            x += SPLIT - GAP
    x0, x1 = 0.5 - total / 2.0 - 0.028, 0.5 + total / 2.0 + 0.028
    img = Image.alpha_composite(img, A._rr(n, x0, BASE_Y + 0.022, x1, BASE_Y + 0.040,
                                           0.009, col))
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
    sheet(keys).save(os.path.join(OUT, "sheet_v10.png"))


if __name__ == "__main__":
    main()
