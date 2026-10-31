# -*- coding: utf-8 -*-
"""图标重设计：5 红 + 2 蓝小球围成一圈，中间「运」字。

三套配色 / 质感方向，只输出预览图，不写入 app/res：
    A 红金   —— 深红漆底 + 金色箍环 + 金色「运」，节庆、厚重
    B 白玉   —— ivory 微暖白底 + 深红字，干净、像系统级图标
    C 暗夜   —— 近黑枣红 + 逆光球体 + 白色「运」，最抓眼

用法：
    python tools/icon_design.py --variant all
    python tools/icon_design.py --variant A --sizes 512,192,48
"""
import os
import math
import argparse
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

APP_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MARK = "\u8fd0"  # 运
FONT_BOLD = r"C:\Windows\Fonts\msyhbd.ttc"

# 几何参数（相对边长）
BALL_R = 0.100
RING_R = 0.298
HOOP_R = 0.400
MARK_PX = 0.245
CY = 0.503
START_DEG = -90.0
BLUE_IDX = (3, 4)

GOLD_TOP, GOLD_BOT = (255, 244, 212), (214, 146, 52)

VARIANTS = {
    "A": dict(
        glows=[(0.26, 0.06, 0.95, (255, 190, 158), 58), (0.50, 0.50, 0.50, (255, 132, 92), 40)],
        hoop=(255, 224, 172, 0), hoop_dark=(72, 2, 3, 0),
        shade=((58, 2, 3), 96),
        red=((255, 144, 114), (216, 28, 18), (255, 140, 100)),
        blue=((160, 208, 255), (36, 122, 250), (130, 194, 255)),
        rim_glow=0.30,
        mark_color=(255, 255, 255), mark_grad=None,
        mark_shadow=((52, 0, 2), 130, 0.007, 0.010),
        center_glow=((255, 246, 236), 58, 0.235), mark_px=0.255,
        edge=(255, 218, 162, 88),
        frame=None, sheen=44,
    ),
    "B": dict(
        glows=[(0.50, 0.30, 0.90, (255, 206, 186), 36)],
        hoop=(255, 255, 255, 0), hoop_dark=(188, 166, 156, 0),
        shade=((150, 62, 50), 72),
        red=((255, 146, 116), (204, 24, 16), (255, 140, 104)),
        blue=((150, 200, 255), (28, 108, 240), (120, 186, 255)),
        rim_glow=0.34,
        mark_color=(190, 22, 14), mark_grad=None,
        mark_shadow=((150, 60, 48), 52, 0.006, 0.010),
        center_glow=((255, 255, 255), 72, 0.250), mark_px=0.255,
        edge=(255, 255, 255, 195),
        frame=None,
    ),
    "C": dict(
        glows=[(0.50, 0.50, 0.64, (255, 96, 62), 64), (0.30, 0.06, 0.72, (255, 152, 122), 26)],
        hoop=(255, 242, 232, 0), hoop_dark=(0, 0, 0, 0),
        shade=((0, 0, 0), 138),
        red=((255, 152, 122), (200, 26, 18), (255, 140, 100)),
        blue=((150, 200, 255), (26, 106, 238), (120, 186, 255)),
        rim_glow=0.46,
        mark_color=(255, 255, 255), mark_grad=None,
        mark_shadow=((0, 0, 0), 160, 0.008, 0.012),
        center_glow=((255, 120, 84), 52, 0.245), mark_px=0.255,
        edge=(255, 255, 255, 38),
        frame=(0.043, 0.190, (255, 226, 208), 40, 0.0038),
    ),
}


# ---------- 基础 ----------

def _grid(n):
    lin = np.linspace(0.0, 1.0, n, dtype=np.float32)
    return np.meshgrid(lin, lin)  # X, Y，形状 (n, n)


def _lerp3(c1, c2, t):
    a = np.asarray(c1, np.float32)
    b = np.asarray(c2, np.float32)
    if a.ndim == 1:
        a = a[None, None, :]
    if b.ndim == 1:
        b = b[None, None, :]
    t = t[..., None]
    return a * (1.0 - t) + b * t


def _smooth(t):
    t = np.clip(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def _glow(n, cx, cy, r, color, alpha, power=1.9):
    X, Y = _grid(n)
    d = np.hypot(X - cx, Y - cy) / r
    a = np.clip(1.0 - d, 0.0, 1.0) ** power * alpha
    return _rgba(n, color, a)


def _disc(n, cx, cy, r, color, alpha, edge=0.22):
    """实心圆盘，边缘柔和收边（比硬边圆更耐看）。"""
    X, Y = _grid(n)
    d = np.hypot(X - cx, Y - cy) / r
    a = _smooth(np.clip((1.0 - d) / edge, 0.0, 1.0)) * alpha
    return _rgba(n, color, a)


def _rgba(n, color, alpha):
    out = np.zeros((n, n, 4), np.uint8)
    out[..., 0], out[..., 1], out[..., 2] = color[:3]
    out[..., 3] = np.clip(alpha, 0, 255).astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def _ball(s, light, base, bounce, rim_glow=0.0):
    """糖果质感球：漫反射 + 锐镜面点 + 柔镜面 + 底部环境反弹光 + 边缘透光。"""
    yy, xx = np.mgrid[0:s, 0:s].astype(np.float32)
    nx = (xx + 0.5) / s * 2.0 - 1.0
    ny = (yy + 0.5) / s * 2.0 - 1.0
    r2 = nx * nx + ny * ny
    inside = r2 <= 1.0
    nz = np.sqrt(np.clip(1.0 - r2, 0.0, 1.0))

    L = np.array([-0.46, -0.58, 0.67], np.float32)
    L /= np.linalg.norm(L)
    ndl = np.clip(nx * L[0] + ny * L[1] + nz * L[2], 0.0, 1.0)
    H = L + np.array([0.0, 0.0, 1.0], np.float32)
    H /= np.linalg.norm(H)
    ndh = np.clip(nx * H[0] + ny * H[1] + nz * H[2], 0.0, 1.0)

    col = np.asarray(base, np.float32)[None, None, :] * (0.34 + 0.92 * ndl)[..., None]
    mix = (np.clip((ndl - 0.46) / 0.54, 0.0, 1.0) ** 1.45 * 0.80)[..., None]
    col = col * (1.0 - mix) + np.asarray(light, np.float32)[None, None, :] * mix
    col *= (0.70 + 0.30 * np.clip(nz, 0, 1) ** 0.45)[..., None]

    # 底部环境反弹光
    bot = (np.clip(ny * 0.95 - nx * 0.30, 0.0, 1.0) ** 1.7) * (np.clip(nz, 0, 1) ** 2.6)
    col += np.asarray(bounce, np.float32)[None, None, :] * bot[..., None] * 0.55

    # 双波瓣镜面
    col += (ndh ** 190)[..., None] * 205.0
    col += (ndh ** 14)[..., None] * 30.0

    if rim_glow > 0:
        fr = np.clip(1.0 - nz, 0.0, 1.0) ** 5.0
        col += fr[..., None] * np.asarray(bounce, np.float32)[None, None, :] * rim_glow

    rgba = np.zeros((s, s, 4), np.uint8)
    rgba[..., :3] = np.clip(col, 0, 255).astype(np.uint8)
    rgba[..., 3] = np.where(inside, 255, 0).astype(np.uint8)
    return Image.fromarray(rgba, "RGBA")


def _ring(n, r, width, color, alpha, blur=0.006):
    layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    c, w = n * 0.5, max(1, int(round(n * width)))
    d.ellipse([c - n * r, c - n * r, c + n * r, c + n * r],
              outline=tuple(color[:3]) + (alpha,), width=w)
    if blur:
        layer = layer.filter(ImageFilter.GaussianBlur(n * blur))
    return layer


def _sheen(n, alpha=44, start=0.02, end=0.62):
    """左上斜向的整片柔光，像玻璃反光，给纯红底一点高级感。"""
    X, Y = _grid(n)
    v = (X * 0.55 + Y * 0.45)
    t = np.clip((end - v) / (end - start), 0.0, 1.0) ** 1.7
    return _rgba(n, (255, 236, 226), t * alpha)


def _frame(n, inset, radius, color, alpha, width, blur=0.0018):
    layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    k = n * inset
    ImageDraw.Draw(layer).rounded_rectangle(
        [k, k, n - 1 - k, n - 1 - k], radius=n * radius,
        outline=tuple(color[:3]) + (alpha,), width=max(1, int(n * width)))
    return layer.filter(ImageFilter.GaussianBlur(n * blur)) if blur else layer


def _emboss_ring(n, r, width, light, la, dark, da):
    """细环压印：下方一条亮线、上方一条暗线，看起来像刻进去的槽。"""
    lay = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    lay = Image.alpha_composite(lay, _ring(n, r - 0.0018, width, light, la, 0.0020))
    lay = Image.alpha_composite(lay, _ring(n, r + 0.0022, width * 1.15, dark, da, 0.0026))
    return lay


def _text(n, ch, cx, cy, px, color=None, grad=None, shadow=None):
    font = ImageFont.truetype(FONT_BOLD, int(round(px)))
    m = Image.new("L", (n, n), 0)
    d = ImageDraw.Draw(m)
    bb = d.textbbox((0, 0), ch, font=font)
    d.text((cx - (bb[2] - bb[0]) / 2 - bb[0], cy - (bb[3] - bb[1]) / 2 - bb[1]),
           ch, font=font, fill=255)

    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    if shadow:
        sc, sa, soff, sblur = shadow
        sh = m.point(lambda v: int(v * sa / 255.0)).filter(
            ImageFilter.GaussianBlur(n * sblur))
        col = Image.new("RGBA", (n, n), tuple(sc[:3]) + (255,))
        col.putalpha(sh)
        out = Image.alpha_composite(out, col.transform(
            (n, n), Image.AFFINE, (1, 0, 0, 0, 1, -soff * n)))

    if grad:
        Y = np.linspace(0.0, 1.0, n, dtype=np.float32)[:, None]
        top = (cy - (bb[3] - bb[1]) / 2.0) / n
        span = max(1e-3, (bb[3] - bb[1]) / float(n))
        t = np.clip((Y - top) / span, 0.0, 1.0)
        t = np.broadcast_to(t, (n, n))
        layer = Image.fromarray(_lerp3(grad[0], grad[1], t).astype(np.uint8), "RGB").convert("RGBA")
    else:
        layer = Image.new("RGBA", (n, n), tuple(color[:3]) + (255,))
    layer.putalpha(m)
    return Image.alpha_composite(out, layer)


def _rounded(n, radius):
    m = Image.new("L", (n, n), 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, n - 1, n - 1], radius=radius, fill=255)
    return m


# ---------- 背景 ----------

BG_HOOK = None


def _bg(n, v):
    if BG_HOOK is not None:
        return BG_HOOK(n)
    X, Y = _grid(n)
    if v == "A":
        # 大半径径向，避免出现明显的"色带圆圈"
        t = _smooth(np.clip(np.hypot((X - 0.28) / 1.35, (Y - 0.10) / 1.35), 0, 1))
        rgb = _lerp3((214, 44, 30), (108, 8, 10), t)
        t2 = _smooth(np.clip(np.hypot((X - 0.5) / 1.05, (Y - 1.06) / 1.05), 0, 1))
        rgb = _lerp3(rgb, (46, 2, 4), t2 * 0.80)
    elif v == "B":
        t = _smooth(np.clip(np.hypot((X - 0.5) / 1.45, (Y - 0.34) / 1.45), 0, 1))
        rgb = _lerp3((255, 255, 255), (245, 235, 230), t)
        t2 = _smooth(np.clip(np.hypot((X - 0.5) / 1.10, (Y - 1.05) / 1.10), 0, 1))
        rgb = _lerp3(rgb, (226, 208, 200), t2 * 0.70)
    else:  # C
        t = _smooth(np.clip(np.hypot((X - 0.40) / 1.45, (Y - 0.30) / 1.45), 0, 1))
        rgb = _lerp3((112, 42, 44), (10, 4, 8), t)
        t2 = _smooth(np.clip(np.hypot((X - 0.5) / 1.05, (Y - 1.05) / 1.05), 0, 1))
        rgb = _lerp3(rgb, (4, 1, 3), t2 * 0.70)
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8), "RGB").convert("RGBA")


# ---------- 主绘制 ----------

def draw(n, v="A"):
    P = VARIANTS[v]
    img = _bg(n, v)

    for g in P["glows"]:
        img = Image.alpha_composite(img, _glow(n, g[0], g[1], g[2], g[3], g[4]))
    if P.get("sheen"):
        img = Image.alpha_composite(img, _sheen(n, P["sheen"]))

    if P.get("disc"):
        dc, da, dr = P["disc"][:3]
        img = Image.alpha_composite(img, _glow(n, 0.5, CY + 0.014, dr * 1.10, (36, 0, 2), 120, 0.55))
        img = Image.alpha_composite(img, _disc(n, 0.5, CY, dr, dc, da, 0.08))

    if P["frame"]:
        f = P["frame"]
        img = Image.alpha_composite(img, _frame(n, f[0], f[1], f[2], f[3], f[4]))

    # 箍环：一圈细环把球"箍"住，构图从散点变成整体
    img = Image.alpha_composite(img, _emboss_ring(
        n, HOOP_R, 0.0045, P["hoop"], P["hoop"][3], P["hoop_dark"], P["hoop_dark"][3]))

    cx, cy = n * 0.5, n * CY
    r = n * BALL_R
    rr = n * RING_R

    spots = []
    for i in range(7):
        a = math.radians(START_DEG + i * 360.0 / 7.0)
        spots.append((cx + rr * math.cos(a), cy + rr * math.sin(a),
                      "blue" if i in BLUE_IDX else "red"))

    # 球与背景之间的落影（统一左上光源 → 右下偏移）
    sh = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ds = ImageDraw.Draw(sh)
    for bx, by, _ in spots:
        ox, oy = r * 0.13, r * 0.17
        ds.ellipse([bx - r * 0.94 + ox, by - r * 0.90 + oy,
                    bx + r * 0.96 + ox, by + r * 1.10 + oy],
                   fill=tuple(P["shade"][0]) + (P["shade"][1],))
    img = Image.alpha_composite(img, sh.filter(ImageFilter.GaussianBlur(n * 0.011)))

    sprites = {}
    for kind in ("red", "blue"):
        light, base, bounce = P[kind]
        sprites[kind] = _ball(int(round(r * 2)), light, base, bounce, P["rim_glow"])
    for bx, by, kind in spots:
        img.alpha_composite(sprites[kind], (int(round(bx - r)), int(round(by - r))))

    # 中心处理：柔光 / 暗井 / 细环
    if P.get("center_glow"):
        gc, ga, gr = P["center_glow"]
        img = Image.alpha_composite(img, _glow(n, 0.5, CY, gr, gc, ga, 1.4))
    if P.get("center_ring"):
        rr_, rc_, ra_ = P["center_ring"]
        img = Image.alpha_composite(img, _ring(n, rr_, 0.0045, rc_, ra_, 0.004))

    # 中心字
    img = Image.alpha_composite(img, _text(
        n, MARK, cx, cy + n * 0.004, n * P.get("mark_px", MARK_PX),
        color=P["mark_color"], grad=P["mark_grad"], shadow=P["mark_shadow"]))

    # 圆角裁切 + 边缘描边
    mask = _rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)

    edge = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(edge).rounded_rectangle(
        [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
        outline=tuple(P["edge"][:3]) + (P["edge"][3],), width=max(1, int(n * 0.009)))
    edge = edge.filter(ImageFilter.GaussianBlur(n * 0.003))
    edge.putalpha(Image.composite(edge.split()[3], Image.new("L", (n, n), 0), mask))
    return Image.alpha_composite(out, edge)


def render(size, v="A"):
    ss = 4 if size <= 256 else 2
    return draw(size * ss, v).resize((size, size), Image.LANCZOS)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--variant", default="all")
    ap.add_argument("--sizes", default="512,192,96,48")
    ap.add_argument("--tag", default="")
    args = ap.parse_args()

    out = os.path.join(APP_DIR, "build", "icon_preview")
    os.makedirs(out, exist_ok=True)
    variants = ["A", "B", "C"] if args.variant.lower() == "all" else [args.variant.upper()]
    sizes = [int(s) for s in args.sizes.split(",")]
    for v in variants:
        for sz in sizes:
            p = os.path.join(out, "d%s%s_%d.png" % (v, args.tag, sz))
            render(sz, v).save(p)
            print(p)


if __name__ == "__main__":
    main()
