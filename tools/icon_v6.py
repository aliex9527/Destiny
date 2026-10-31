# -*- coding: utf-8 -*-
"""第六轮图标：不动几何参数，换结构。

前五轮的问题不是球太多，而是**结构只有一种**（球环 + 中心字），
再怎么调 BALL_R / RING_R 都还在同一个抽屉里。这一轮做四个结构完全不同的方案，
每个方案只保留「一个主形状 + 最多一个点缀」，纯色、无渐变、无光效。

  N1 靶心     深红底 + 白粗环 + 白实心点        命中
  N2 趋势球   深红底 + 白大球 + 球内红色折线    智算
  N3 双区     米白底 + 红球与蓝球相切并列       前区 / 后区
  N4 5+2      深红底 + 白色「5+2」               玩法本身

用法：
    python tools/icon_v6.py                      # 全部，512/192/96/48
    python tools/icon_v6.py --only N1,N3 --sizes 512,48
"""
import os
import argparse
from PIL import Image, ImageDraw, ImageFont

import icon_design as D

APP_DIR = D.APP_DIR
OUT = os.path.join(APP_DIR, "build", "icon_v6")

RED = (196, 23, 15)
RED_DEEP = (168, 14, 10)
BLUE = (26, 106, 238)
WHITE = (255, 253, 250)
IVORY = (250, 246, 240)


# ---------- 基础图元（全部硬边，靠 4x 超采样拿抗锯齿） ----------

def _flat(n, color):
    return Image.new("RGBA", (n, n), tuple(color) + (255,))


def _disc(n, cx, cy, r, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).ellipse(
        [n * cx - n * r, n * cy - n * r, n * cx + n * r, n * cy + n * r],
        fill=tuple(color) + (255,))
    return lay


def _ring(n, cx, cy, r, w, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).ellipse(
        [n * cx - n * r, n * cy - n * r, n * cx + n * r, n * cy + n * r],
        outline=tuple(color) + (255,), width=max(1, int(round(n * w))))
    return lay


def _polyline(n, pts, w, color, caps=True):
    """粗折线，圆角接头 + 圆头端点。"""
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay)
    xy = [(n * x, n * y) for x, y in pts]
    ww = max(2, int(round(n * w)))
    d.line(xy, fill=tuple(color) + (255,), width=ww, joint="curve")
    if caps:
        for x, y in (xy[0], xy[-1]):
            d.ellipse([x - ww / 2.0, y - ww / 2.0, x + ww / 2.0, y + ww / 2.0],
                      fill=tuple(color) + (255,))
    return lay


def _text(n, s, cx, cy, ratio, color, track=0.0):
    """多字符排版：ratio 为字号占边长的比例，最后按整体墨迹居中。"""
    size = max(1, int(round(n * ratio)))
    font = ImageFont.truetype(D.FONT_BOLD, size)
    m = Image.new("L", (n, n), 0)
    d = ImageDraw.Draw(m)
    widths = [d.textbbox((0, 0), ch, font=font)[2] for ch in s]
    gap = size * track
    total = sum(widths) + gap * (len(s) - 1)
    x = (n - total) / 2.0
    for i, ch in enumerate(s):
        bb = d.textbbox((0, 0), ch, font=font)
        d.text((x - bb[0], 0), ch, font=font, fill=255)
        x += widths[i] + gap
    bb2 = m.getbbox()
    if bb2:
        dx = n * cx - (bb2[0] + bb2[2]) / 2.0
        dy = n * cy - (bb2[1] + bb2[3]) / 2.0
        m = m.transform((n, n), Image.AFFINE, (1, 0, -dx, 0, 1, -dy))
    layer = Image.new("RGBA", (n, n), tuple(color) + (255,))
    layer.putalpha(m)
    return layer


def _finish(n, img, hairline=False):
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    if hairline:
        lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        ImageDraw.Draw(lay).rounded_rectangle(
            [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
            outline=(0, 0, 0, 30), width=max(1, int(n * 0.007)))
        lay.putalpha(Image.composite(lay.split()[3], Image.new("L", (n, n), 0), mask))
        out = Image.alpha_composite(out, lay)
    return out


# ---------- 四个结构 ----------

def n1(n):
    """靶心：深红底 + 白粗环 + 白点。只有一个母题，48px 也不糊。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _ring(n, 0.5, 0.5, 0.306, 0.078, WHITE))
    img = Image.alpha_composite(img, _disc(n, 0.5, 0.5, 0.100, WHITE))
    return _finish(n, img)


def n2(n):
    """趋势球：白色大球里画一根红色上升折线，球即「号」，线即「算」。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _disc(n, 0.5, 0.5, 0.340, WHITE))
    img = Image.alpha_composite(img, _polyline(
        n, [(0.318, 0.636), (0.430, 0.534), (0.522, 0.598), (0.672, 0.386)],
        0.058, RED))
    return _finish(n, img)


def n3(n):
    """双区：红球大、蓝球小，相切并列。5 红 2 蓝的关系用尺寸差表达。"""
    img = _flat(n, IVORY)
    img = Image.alpha_composite(img, _disc(n, 0.376, 0.5, 0.248, RED))
    img = Image.alpha_composite(img, _disc(n, 0.694, 0.5, 0.158 + 0.014, IVORY))
    img = Image.alpha_composite(img, _disc(n, 0.694, 0.5, 0.158, BLUE))
    return _finish(n, img, hairline=True)


def n4(n):
    """5+2：直接把玩法写在图标上。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _text(n, "5+2", 0.5, 0.5, 0.345, WHITE, track=-0.02))
    return _finish(n, img)


def n5(n):
    """柱球：白球里三根上升柱。比折线更抗缩放，缩到 48px 也还是「三根柱子」。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _disc(n, 0.5, 0.5, 0.340, WHITE))
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay)
    w, gap = 0.076, 0.030
    left = 0.5 - (3 * w + 2 * gap) / 2.0
    for i, h in enumerate((0.112, 0.184, 0.256)):
        x = left + i * (w + gap)
        d.rounded_rectangle([n * x, n * (0.660 - h), n * (x + w), n * 0.660],
                            radius=n * w / 2.0, fill=RED + (255,))
    img = Image.alpha_composite(img, lay)
    return _finish(n, img)


FNS = {"N1": n1, "N2": n2, "N3": n3, "N4": n4, "N5": n5}
NAMES = {"N1": "靶心", "N2": "趋势球", "N3": "双区", "N4": "5+2", "N5": "柱球"}


def render(size, key):
    ss = 4 if size <= 256 else 2
    return FNS[key](size * ss).resize((size, size), Image.LANCZOS)


def sheet(keys, sizes=(256, 96, 48)):
    """汇总图：每个方案一行，三档尺寸并排，模拟桌面上的真实观感。"""
    pad, gap = 28, 24
    cols = [0] * len(sizes)
    for i, s in enumerate(sizes):
        cols[i] = s
    w = pad * 2 + sum(cols) + gap * (len(cols) - 1) + 200
    h = pad * 2 + len(keys) * (max(sizes) + sum(s for s in sizes[1:]) // len(sizes) + gap)
    h = pad * 2 + len(keys) * (max(cols) + gap)
    img = Image.new("RGBA", (int(w), int(h)), (238, 238, 240, 255))
    d = ImageDraw.Draw(img)
    try:
        f = ImageFont.truetype(D.FONT_BOLD, 20)
    except Exception:
        f = None
    for r, k in enumerate(keys):
        y = pad + r * (max(cols) + gap)
        d.text((pad - 6, y + 6), "%s %s" % (k, NAMES[k]), font=f, fill=(60, 60, 66, 255))
        x = pad + 200
        for sz in sizes:
            icon = render(sz, k)
            img.alpha_composite(icon, (int(x), int(y + (max(cols) - sz) / 2)))
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
    sheet(keys).save(os.path.join(OUT, "sheet_v6.png"))
    print(os.path.join(OUT, "sheet_v6.png"))


if __name__ == "__main__":
    main()
