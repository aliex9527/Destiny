# -*- coding: utf-8 -*-
"""把定稿方案导出成可直接使用的图片。

  · 1024 / 512 / 192 / 144 / 96 / 72 / 48 七个尺寸
  · 一张汇总效果图（大图 + 三档小尺寸对照）
"""
import os
import argparse
from PIL import Image, ImageDraw, ImageFont

import icon_design as D
import icon_v11 as A

OUT = os.path.join(D.APP_DIR, "build", "icon_final")
SIZES = (1024, 512, 192, 144, 96, 72, 48)


def render(size, key):
    ss = 2 if size > 512 else (4 if size <= 256 else 2)
    return A.FNS[key](size * ss).resize((size, size), Image.LANCZOS)


def showcase(key, big=340):
    pad, gap = 64, 40
    small = (96, 72, 48)
    w = pad * 2 + big + gap * 2 + sum(small) + gap * (len(small) - 1)
    h = pad * 2 + max(big, 220)
    img = Image.new("RGBA", (w, h), (243, 244, 247, 255))
    d = ImageDraw.Draw(img)
    fb = ImageFont.truetype(D.FONT_BOLD, 26)
    fs = ImageFont.truetype(D.FONT_BOLD, 18)

    img.alpha_composite(render(big, key), (pad, pad + (h - pad * 2 - big) // 2))
    d.text((pad, pad - 34), "定稿 512 原尺寸", font=fs, fill=(120, 124, 132, 255))

    x = pad + big + gap * 2
    y = pad + (h - pad * 2 - 220) // 2
    d.text((x, y - 30), "桌面实际尺寸", font=fs, fill=(120, 124, 132, 255))
    for sz in small:
        icon = render(sz, key)
        img.alpha_composite(icon, (int(x), int(y + (96 - sz) // 2 + 30)))
        d.text((x, y + 140), "%dpx" % sz, font=fb, fill=(70, 74, 82, 255))
        x += sz + gap
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--key", default="B1")
    args = ap.parse_args()
    os.makedirs(OUT, exist_ok=True)
    for sz in SIZES:
        p = os.path.join(OUT, "%s_%d.png" % (args.key, sz))
        render(sz, args.key).save(p)
        print(p)
    p = os.path.join(OUT, "%s_showcase.png" % args.key)
    showcase(args.key).save(p)
    print(p)


if __name__ == "__main__":
    main()
