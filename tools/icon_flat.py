# -*- coding: utf-8 -*-
"""按 App 图标设计规范重做：一个主形状、2~3 色、无多余光效。

参考要点（Apple HIG / 2026 图标趋势）：
  · 一个视觉核心，最多两个几何元素
  · 纯色底，让主体说话；不做多层光晕/描边/接触阴影
  · 先做单色能否成立，再加第二色
  · 任何小于画布 10% 的细节，缩小后都等于噪点

六个方案：
  F1 印章     红底 + 白圆 + 红「运」
  F2 白环     红底 + 白环 + 白「运」
  F3 双球     红底 + 大白球 + 小蓝球
  F4 球上字下 红底 + 白球 + 白「运」
  F5 珠环     红底 + 白环串珠（5 白 2 蓝）+ 白「运」
  F6 球中字   红底 + 带体积的白球 + 球内红「运」+ 蓝球
"""
import os
import sys
import math
import argparse
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import icon_design as D  # noqa: E402  复用文字渲染与圆角遮罩

OUT = os.path.join(D.APP_DIR, "build", "icon_preview")

# 品牌红（纯色，不做渐变）
RED = (196, 23, 15)
RED_DEEP = (168, 14, 10)
WHITE = (255, 253, 250)
BLUE = (28, 108, 238)

MARK = D.MARK


def _grid(n):
    lin = np.linspace(0.0, 1.0, n, dtype=np.float32)
    return np.meshgrid(lin, lin)


def _flat(n, color):
    out = np.zeros((n, n, 4), np.uint8)
    out[..., 0], out[..., 1], out[..., 2] = color
    out[..., 3] = 255
    return Image.fromarray(out, "RGBA")


def _sphere(n, cx, cy, r, base, hi=None, gloss=0.0):
    """克制的球：左上受光，中心微亮，边缘略暗。没有多层光晕。"""
    X, Y = _grid(n)
    dx = (X - cx) / r
    dy = (Y - cy) / r
    d2 = dx * dx + dy * dy
    inside = d2 <= 1.0
    nz = np.sqrt(np.clip(1.0 - d2, 0.0, 1.0))

    L = np.array([-0.45, -0.55, 0.70], np.float32)
    L /= np.linalg.norm(L)
    ndl = np.clip(dx * L[0] + dy * L[1] + nz * L[2], 0.0, 1.0)

    k = 0.87 + 0.15 * ndl
    col = np.asarray(base, np.float32)[None, None, :] * k[..., None]
    col *= (0.955 + 0.045 * nz)[..., None]
    if hi is not None:
        col = col * (1 - 0.18 * ndl[..., None]) + np.asarray(hi, np.float32) * (0.18 * ndl[..., None])
    if gloss > 0:
        col += (ndl ** 24)[..., None] * gloss

    rgba = np.zeros((n, n, 4), np.uint8)
    rgba[..., :3] = np.clip(col, 0, 255).astype(np.uint8)
    rgba[..., 3] = np.where(inside, 255, 0).astype(np.uint8)
    return Image.fromarray(rgba, "RGBA")


def _disc(n, cx, cy, r, color):
    X, Y = _grid(n)
    d = np.hypot(X - cx, Y - cy) / r
    a = np.clip((1.0 - d) * r * n * 0.5, 0.0, 1.0) * 255
    out = np.zeros((n, n, 4), np.uint8)
    out[..., 0], out[..., 1], out[..., 2] = color
    out[..., 3] = a.astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def _ring(n, cx, cy, r, w, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    c, rr = n * cx, n * r
    ww = max(1, int(round(n * w)))
    ImageDraw.Draw(lay).ellipse([c - rr, n * cy - rr, c + rr, n * cy + rr],
                                outline=color + (255,), width=ww)
    return lay


def _arc(n, cx, cy, r, start, end, w, color):
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    c = n * cx
    ImageDraw.Draw(lay).arc([c - n * r, n * cy - n * r, c + n * r, n * cy + n * r],
                            start, end, fill=color + (255,), width=max(1, int(n * w)))
    return lay


def _text(n, cx, cy, px, color):
    return D._text(n, MARK, n * cx, n * cy, n * px, color=color, grad=None, shadow=None)


def _finish(n, img, hairline=False):
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    if hairline:
        lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        ImageDraw.Draw(lay).rounded_rectangle(
            [n * 0.005, n * 0.005, n * 0.995, n * 0.995], radius=n * 0.225,
            outline=(0, 0, 0, 26), width=max(1, int(n * 0.006)))
        out = Image.alpha_composite(out, lay.filter(ImageFilter.GaussianBlur(n * 0.002)))
        out.putalpha(Image.composite(out.split()[3], Image.new("L", (n, n), 0), mask))
    return out


# ---------- 六个方案 ----------

def f1(n):
    """印章：红底 + 白圆 + 红字。"""
    img = _flat(n, RED)
    img = Image.alpha_composite(img, _ring(n, 0.5, 0.5, 0.372, 0.011, WHITE))
    img = Image.alpha_composite(img, _disc(n, 0.5, 0.5, 0.332, WHITE))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.352, RED))
    return _finish(n, img)


def f2(n):
    """白环 + 白字，最轻的一版。"""
    img = _flat(n, RED)
    img = Image.alpha_composite(img, _ring(n, 0.5, 0.5, 0.322, 0.030, WHITE))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.330, WHITE))
    return _finish(n, img)


def f3(n):
    """双球：大白球 + 小蓝球，无字。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _sphere(n, 0.428, 0.420, 0.278, WHITE, gloss=70))
    img = Image.alpha_composite(img, _sphere(n, 0.688, 0.690, 0.140, BLUE, gloss=45))
    return _finish(n, img)


def f4(n):
    """白球在上，字在下。"""
    img = _flat(n, RED)
    img = Image.alpha_composite(img, _sphere(n, 0.5, 0.352, 0.195, WHITE, gloss=60))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.722, 0.295, WHITE))
    return _finish(n, img)


def f5(n):
    """白环串珠：5 白 2 蓝，环形保持，但做成平面点，不带立体。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _ring(n, 0.5, 0.5, 0.300, 0.016, WHITE))
    r = 0.063
    for i in range(7):
        a = math.radians(-90 + i * 360.0 / 7.0)
        cx = 0.5 + 0.300 * math.cos(a)
        cy = 0.5 + 0.300 * math.sin(a)
        col = BLUE if i in (3, 4) else WHITE
        img = Image.alpha_composite(img, _disc(n, cx, cy, r, col))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.225, WHITE))
    return _finish(n, img)


def f6(n):
    """白球里挖出红字，右下角一颗蓝球。"""
    img = _flat(n, RED_DEEP)
    img = Image.alpha_composite(img, _sphere(n, 0.462, 0.450, 0.310, WHITE, gloss=60))
    img = Image.alpha_composite(img, _sphere(n, 0.722, 0.756, 0.118, BLUE, gloss=45))
    img = Image.alpha_composite(img, _text(n, 0.462, 0.458, 0.268, RED_DEEP))
    return _finish(n, img)


def f7(n):
    """白底 + 7 颗红蓝圆点围环 + 红「运」。"""
    img = _flat(n, (255, 253, 251))
    r, rr = 0.060, 0.302
    for i in range(7):
        a = math.radians(-90 + i * 360.0 / 7.0)
        cx = 0.5 + rr * math.cos(a)
        cy = 0.5 + rr * math.sin(a)
        img = Image.alpha_composite(img, _disc(n, cx, cy, r, BLUE if i in (3, 4) else RED))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.212, RED))
    return _finish(n, img, hairline=True)


def f8(n):
    """白底 + 一个粗红环 + 红「运」，只有一个形状加一个字。"""
    img = _flat(n, (255, 253, 251))
    img = Image.alpha_composite(img, _ring(n, 0.5, 0.5, 0.298, 0.042, RED))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.262, RED))
    return _finish(n, img, hairline=True)


def f9(n):
    """红底 + 7 颗白点（2 蓝）围环 + 白「运」，不画环线。"""
    img = _flat(n, RED_DEEP)
    r, rr = 0.060, 0.306
    for i in range(7):
        a = math.radians(-90 + i * 360.0 / 7.0)
        cx = 0.5 + rr * math.cos(a)
        cy = 0.5 + rr * math.sin(a)
        img = Image.alpha_composite(img, _disc(n, cx, cy, r, BLUE if i in (3, 4) else WHITE))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.262, WHITE))
    return _finish(n, img)


def f11(n):
    """白底 + 7 颗红点（2 蓝）围环 + 红「运」，点更小、环更大。"""
    img = _flat(n, (255, 253, 251))
    r, rr = 0.058, 0.308
    for i in range(7):
        a = math.radians(-90 + i * 360.0 / 7.0)
        cx = 0.5 + rr * math.cos(a)
        cy = 0.5 + rr * math.sin(a)
        img = Image.alpha_composite(img, _disc(n, cx, cy, r, BLUE if i in (3, 4) else RED))
    img = Image.alpha_composite(img, _text(n, 0.5, 0.505, 0.256, RED))
    return _finish(n, img, hairline=True)


def f10(n):
    """白底：红球 + 蓝球，无字。"""
    img = _flat(n, (255, 253, 251))
    img = Image.alpha_composite(img, _sphere(n, 0.412, 0.404, 0.280, RED, hi=(236, 78, 60), gloss=70))
    img = Image.alpha_composite(img, _sphere(n, 0.700, 0.706, 0.138, BLUE, gloss=45))
    return _finish(n, img, hairline=True)


def f12(n):
    """红底 + 白色大「运」+ 右下角一颗蓝球。"""
    img = _flat(n, RED)
    img = Image.alpha_composite(img, _text(n, 0.452, 0.448, 0.440, WHITE))
    img = Image.alpha_composite(img, _sphere(n, 0.796, 0.796, 0.090, BLUE, gloss=45))
    return _finish(n, img)


def f13(n):
    """白底 + 红色大「运」+ 右下角一颗蓝球。"""
    img = _flat(n, (255, 253, 251))
    img = Image.alpha_composite(img, _text(n, 0.452, 0.448, 0.440, RED))
    img = Image.alpha_composite(img, _sphere(n, 0.796, 0.796, 0.090, BLUE, gloss=45))
    return _finish(n, img, hairline=True)


FNS = {"F12": f12, "F13": f13, "F9": f9, "F10": f10, "F11": f11, "F1": f1, "F2": f2, "F3": f3, "F4": f4, "F5": f5, "F6": f6,
       "F7": f7, "F8": f8}
NAMES = {"F1": "印章", "F2": "白环", "F3": "双球", "F4": "球上字下",
         "F5": "珠环", "F6": "球中字", "F7": "白底彩珠", "F8": "白底红环", "F9": "白点珠环", "F10": "白底双球", "F11": "白底红点环", "F12": "红底大字", "F13": "白底大字"}


def render(size, key):
    ss = 4 if size <= 256 else 2
    return FNS[key](size * ss).resize((size, size), Image.LANCZOS)


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


if __name__ == "__main__":
    main()
