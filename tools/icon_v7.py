# -*- coding: utf-8 -*-
"""第七轮图标：科技 / 数据感，彻底不用「球」。

前六轮全部围绕「球 + 环 + 中心字」，同一套符号怎么调都像一个抽屉里的东西。
这一轮改用数据可视化里的四种基本形式，深色底、单一亮色、硬边、无光效：

  T1 数据柱   7 根高低不一的柱，5 白 2 蓝            前区 / 后区就是一组数据
  T2 分布曲线 钟形正态曲线 + 峰顶一个蓝点            智算 = 概率
  T3 热力矩阵 4x4 圆角格，按热度填色                 冷热号
  T4 轨道     一道蓝环 + 环上停着一颗白球            概率轨道 / 命中

用法：
    python tools/icon_v7.py
    python tools/icon_v7.py --only T1,T2 --sizes 512,48
"""
import os
import math
import argparse
import numpy as np
from PIL import Image, ImageDraw, ImageFont

import icon_design as D

APP_DIR = D.APP_DIR
OUT = os.path.join(APP_DIR, "build", "icon_v7")

INK = (18, 21, 28)          # 近黑底，比纯黑温和
INK_2 = (38, 44, 56)        # 未点亮的格子
WHITE = (242, 245, 249)
BLUE = (59, 130, 246)       # 电蓝
CYAN = (45, 212, 191)


# ---------- 图元 ----------

def _flat(n, color):
    return Image.new("RGBA", (n, n), tuple(color) + (255,))


def _rr(n, x0, y0, x1, y1, rad, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).rounded_rectangle(
        [n * x0, n * y0, n * x1, n * y1], radius=n * rad, fill=tuple(color) + (255,))
    return lay


def _disc(n, cx, cy, r, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(lay).ellipse(
        [n * cx - n * r, n * cy - n * r, n * cx + n * r, n * cy + n * r],
        fill=tuple(color) + (255,))
    return lay


def _ring(n, cx, cy, r, w, color, start=None, end=None):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    box = [n * cx - n * r, n * cy - n * r, n * cx + n * r, n * cy + n * r]
    ww = max(1, int(round(n * w)))
    d = ImageDraw.Draw(lay)
    if start is None:
        d.ellipse(box, outline=tuple(color) + (255,), width=ww)
    else:
        d.arc(box, start, end, fill=tuple(color) + (255,), width=ww)
    return lay


def _line(n, pts, w, color, caps=True, joint="curve"):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay)
    xy = [(n * x, n * y) for x, y in pts]
    ww = max(2, int(round(n * w)))
    d.line(xy, fill=tuple(color) + (255,), width=ww, joint=joint)
    if caps:
        for x, y in (xy[0], xy[-1]):
            d.ellipse([x - ww / 2.0, y - ww / 2.0, x + ww / 2.0, y + ww / 2.0],
                      fill=tuple(color) + (255,))
    return lay


def _stroke(n, pts, w, color):
    """粗曲线：沿路径逐点采样圆盘再统一上色。PIL 的 joint="curve" 会在线段接头处爆刺。"""
    m = Image.new("L", (n, n), 0)
    d = ImageDraw.Draw(m)
    rr = max(1.0, n * w / 2.0)
    for x, y in pts:
        d.ellipse([n * x - rr, n * y - rr, n * x + rr, n * y + rr], fill=255)
    lay = Image.new("RGBA", (n, n), tuple(color) + (255,))
    lay.putalpha(m)
    return lay


def _finish(n, img, hairline=False):
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    if hairline:
        lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        ImageDraw.Draw(lay).rounded_rectangle(
            [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
            outline=(255, 255, 255, 34), width=max(1, int(n * 0.007)))
        lay.putalpha(Image.composite(lay.split()[3], Image.new("L", (n, n), 0), mask))
        out = Image.alpha_composite(out, lay)
    return out


# ---------- 四个结构 ----------

def t1(n):
    """数据柱：7 根柱 = 5 白（前区）+ 2 蓝（后区），高度是「号码热度」。"""
    img = _flat(n, INK)
    w, gap = 0.062, 0.024
    left = 0.5 - (7 * w + 6 * gap) / 2.0
    base = 0.672
    hs = (0.170, 0.288, 0.230, 0.372, 0.276, 0.208, 0.330)
    for i, h in enumerate(hs):
        x = left + i * (w + gap)
        col = BLUE if i >= 5 else WHITE
        img = Image.alpha_composite(img, _rr(n, x, base - h, x + w, base, w / 2.0, col))
    return _finish(n, img, hairline=True)


def t2(n):
    """分布曲线：正态钟形 + 曲线下淡填充。不加圆点，避免看成一双眼睛。"""
    img = _flat(n, INK)
    xs = np.linspace(0.098, 0.902, 200)
    mu, sig, amp, base_y = 0.5, 0.148, 0.410, 0.700
    ys = base_y - amp * np.exp(-((xs - mu) ** 2) / (2 * sig ** 2))

    poly = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    pts = [(n * xs[0], n * base_y)] + [(n * x, n * y) for x, y in zip(xs, ys)] + \
          [(n * xs[-1], n * base_y)]
    ImageDraw.Draw(poly).polygon(pts, fill=WHITE + (30,))
    img = Image.alpha_composite(img, poly)

    img = Image.alpha_composite(img, _stroke(n, list(zip(xs.tolist(), ys.tolist())), 0.042, WHITE))
    return _finish(n, img, hairline=True)


SEGS = {"0": "abcdef", "2": "abged", "5": "afgcd", "7": "abc", "8": "abcdefg"}


def _seg_digit(n, ch, x0, y0, H, W, t, color):
    """七段数码管造型的一个字。x0/y0 为左上角，H/W 为字高宽，t 为笔画粗。"""
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay)
    g = t * 0.55
    xm, ym = x0 + W / 2.0, y0 + H / 2.0
    bars = {
        "a": (x0 + g, y0, x0 + W - g, y0 + t),
        "d": (x0 + g, y0 + H - t, x0 + W - g, y0 + H),
        "g": (x0 + g, ym - t / 2.0, x0 + W - g, ym + t / 2.0),
        "f": (x0, y0 + g, x0 + t, ym - g),
        "b": (x0 + W - t, y0 + g, x0 + W, ym - g),
        "e": (x0, ym + g, x0 + t, y0 + H - g),
        "c": (x0 + W - t, ym + g, x0 + W, y0 + H - g),
    }
    if ch == "+":
        # 加号必须比数字矮一截、横臂更长，否则会被读成「1」
        hw, vh = W * 0.86, t * 2.5
        d.rounded_rectangle([n * (xm - hw / 2.0), n * (ym - t / 2.0),
                             n * (xm + hw / 2.0), n * (ym + t / 2.0)],
                            radius=n * t / 2.0, fill=tuple(color) + (255,))
        d.rounded_rectangle([n * (xm - t / 2.0), n * (ym - vh / 2.0),
                             n * (xm + t / 2.0), n * (ym + vh / 2.0)],
                            radius=n * t / 2.0, fill=tuple(color) + (255,))
        return lay
    for k in SEGS.get(ch, ""):
        b = bars[k]
        d.rounded_rectangle([n * b[0], n * b[1], n * b[2], n * b[3]],
                            radius=n * t / 2.0, fill=tuple(color) + (255,))
    return lay


def t3(n):
    """数码「5+2」：七段管造型，像一块数据终端。玩法本身成了一组读数。"""
    img = _flat(n, INK)
    H, Wd, t = 0.420, 0.230, 0.056
    Wp, gap = 0.135, 0.070
    total = Wd + gap + Wp + gap + Wd
    x = 0.5 - total / 2.0
    y0 = 0.5 - H / 2.0
    for ch, w in (("5", Wd), ("+", Wp), ("2", Wd)):
        col = BLUE if ch == "2" else WHITE
        img = Image.alpha_composite(img, _seg_digit(n, ch, x, y0, H, w, t, col))
        x += w + gap
    return _finish(n, img, hairline=True)


def t4(n):
    """轨道：一道蓝环，环上停一颗白球。环留一个缺口，避免变成「loading」。"""
    img = _flat(n, INK)
    img = Image.alpha_composite(img, _ring(n, 0.5, 0.5, 0.300, 0.040, BLUE,
                                           start=-58, end=272))
    ang = math.radians(-58)
    img = Image.alpha_composite(img, _disc(
        n, 0.5 + 0.300 * math.cos(ang), 0.5 + 0.300 * math.sin(ang), 0.098, WHITE))
    return _finish(n, img, hairline=True)


FNS = {"T1": t1, "T2": t2, "T3": t3, "T4": t4}
NAMES = {"T1": "数据柱", "T2": "分布曲线", "T3": "数码5+2", "T4": "轨道"}


def render(size, key):
    ss = 4 if size <= 256 else 2
    return FNS[key](size * ss).resize((size, size), Image.LANCZOS)


def sheet(keys, sizes=(256, 96, 48)):
    pad, gap, label = 28, 24, 200
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
    sheet(keys).save(os.path.join(OUT, "sheet_v7.png"))


if __name__ == "__main__":
    main()
