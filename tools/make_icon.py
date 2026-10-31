# -*- coding: utf-8 -*-
"""生成「大乐透智算」启动图标。

设计：浅红渐变圆角底，压一颗深红立体球（前区红球），球面白色品牌字「乐」，
右下角叠一颗蓝色小球（后区蓝球）。与应用内 header 的 logo 同源，保持品牌一致。

全部 4x 超采样后 LANCZOS 降采样，输出 5 档密度到 app/res/mipmap-*/ic_launcher.png。

用法：
    python tools/make_icon.py              # 写入 app/res/mipmap-*
    python tools/make_icon.py --preview    # 只输出预览图到 build/icon_preview
"""
import os
import math
import argparse
from PIL import Image, ImageDraw, ImageFilter, ImageFont

SS = 4  # 超采样倍数
APP_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GLYPH = "\u4e50"  # 乐


# ---------- 基础工具 ----------

def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(len(a)))


def rounded_mask(n, radius):
    m = Image.new("L", (n, n), 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, n - 1, n - 1], radius=radius, fill=255)
    return m


def linear_gradient(n, c1, c2, angle=128):
    rad = math.radians(angle)
    dx, dy = math.cos(rad), math.sin(rad)
    proj = [0, n * dx, n * dy, n * dx + n * dy]
    lo, hi = min(proj), max(proj)
    img = Image.new("RGB", (n, n))
    px = img.load()
    for y in range(n):
        row = []
        for x in range(n):
            t = ((x * dx + y * dy) - lo) / (hi - lo)
            row.append(lerp(c1, c2, max(0.0, min(1.0, t))))
        for x, c in enumerate(row):
            px[x, y] = c
    return img


def radial_glow(n, cx, cy, r, color, alpha, power=1.6):
    g = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(g)
    steps = 28
    for i in range(steps, 0, -1):
        t = i / steps
        rr = r * t
        a = int(alpha * (1 - t) ** power)
        if a <= 0:
            continue
        d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], fill=color + (a,))
    return g.filter(ImageFilter.GaussianBlur(n * 0.035))


def sphere(size, light, base, dark, rim_color=None, rim_w=0.017, gloss_alpha=110):
    """左上受光的立体球。"""
    s = int(round(size))
    grad = Image.new("RGB", (s, s), dark)
    gp = grad.load()
    lx, ly = s * 0.33, s * 0.28
    maxd = math.hypot(s * 0.5 - lx, s * 0.5 - ly) + s * 0.42
    for y in range(s):
        dy2 = y - ly
        for x in range(s):
            t = min(1.0, math.hypot(x - lx, dy2) / maxd)
            gp[x, y] = (lerp(light, base, t / 0.42) if t < 0.42
                        else lerp(base, dark, (t - 0.42) / 0.58))

    ball_mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(ball_mask).ellipse([0, 0, s - 1, s - 1], fill=255)

    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    img.paste(grad, (0, 0), ball_mask)

    if gloss_alpha > 0:
        # 大面柔光
        gl = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        ImageDraw.Draw(gl).ellipse([s * 0.16, s * 0.06, s * 0.82, s * 0.46],
                                   fill=(255, 255, 255, int(gloss_alpha * 0.8)))
        gl = gl.filter(ImageFilter.GaussianBlur(s * 0.07))
        # 底部反射光
        bo = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        ImageDraw.Draw(bo).arc([s * 0.08, s * 0.08, s * 0.92, s * 0.92], 30, 150,
                               fill=(255, 255, 255, 58), width=max(1, int(s * 0.05)))
        bo = bo.filter(ImageFilter.GaussianBlur(s * 0.03))
        for layer in (gl, bo):
            layer.putalpha(Image.composite(layer.split()[3],
                                           Image.new("L", (s, s), 0), ball_mask))
            img = Image.alpha_composite(img, layer)

    if rim_color:
        ring = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        w = max(1, int(s * rim_w))
        ImageDraw.Draw(ring).ellipse([w / 2, w / 2, s - 1 - w / 2, s - 1 - w / 2],
                                     outline=rim_color, width=w)
        img = Image.alpha_composite(img, ring)
    return img


def find_font():
    for p in (r"C:\Windows\Fonts\msyhbd.ttc", r"C:\Windows\Fonts\msyh.ttc",
              r"C:\Windows\Fonts\simhei.ttf"):
        if os.path.exists(p):
            return p
    raise SystemExit("未找到中文字体")


def draw_glyph(img, n, ch, cx, cy, px, color):
    """按墨迹包围盒精确居中绘制汉字。"""
    font = ImageFont.truetype(find_font(), int(px))
    layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    bb = d.textbbox((0, 0), ch, font=font)
    d.text((cx - (bb[2] - bb[0]) / 2 - bb[0], cy - (bb[3] - bb[1]) / 2 - bb[1]),
           ch, font=font, fill=color)
    return Image.alpha_composite(img, layer)


# ---------- 图标绘制 ----------

def draw_icon(n):
    radius = n * 0.228

    # 底色：浅红渐变 + 左上光晕 + 右下压暗
    img = linear_gradient(n, (255, 118, 84), (188, 16, 9), 128).convert("RGBA")
    img = Image.alpha_composite(img, radial_glow(n, n * 0.24, n * 0.08, n * 0.95,
                                                 (255, 255, 255), 62))
    deep = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(deep).ellipse([n * 0.48, n * 0.48, n * 1.35, n * 1.35], fill=(90, 4, 2, 80))
    img = Image.alpha_composite(img, deep.filter(ImageFilter.GaussianBlur(n * 0.15)))

    br, bcx, bcy = n * 0.335, n * 0.443, n * 0.437
    sr, scx, scy = n * 0.142, n * 0.756, n * 0.762

    # 落影
    sh = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(sh)
    d.ellipse([bcx - br * 0.97, bcy - br * 0.74, bcx + br * 1.03, bcy + br * 1.16],
              fill=(118, 6, 3, 150))
    d.ellipse([scx - sr * 0.92, scy - sr * 0.70, scx + sr * 1.08, scy + sr * 1.16],
              fill=(60, 3, 1, 110))
    img = Image.alpha_composite(img, sh.filter(ImageFilter.GaussianBlur(n * 0.024)))

    # 前区红球 + 品牌字
    img.alpha_composite(sphere(br * 2, (255, 138, 112), (222, 44, 30), (150, 12, 6),
                               rim_color=(255, 226, 216, 105), rim_w=0.012, gloss_alpha=118),
                        (int(round(bcx - br)), int(round(bcy - br))))
    img = draw_glyph(img, n, GLYPH, bcx, bcy + n * 0.006, n * 0.425, (255, 255, 255, 255))

    # 蓝球对红球的接触阴影
    c = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ccx, ccy = scx - n * 0.016, scy - n * 0.016
    ImageDraw.Draw(c).ellipse([ccx - sr * 1.05, ccy - sr * 1.05, ccx + sr * 1.05, ccy + sr * 1.05],
                              fill=(104, 8, 4, 135))
    img = Image.alpha_composite(img, c.filter(ImageFilter.GaussianBlur(n * 0.017)))

    # 后区蓝球（在前）
    img.alpha_composite(sphere(sr * 2, (140, 196, 255), (46, 124, 250), (10, 50, 152),
                               rim_color=(255, 250, 248, 205), rim_w=0.013, gloss_alpha=118),
                        (int(round(scx - sr)), int(round(scy - sr))))

    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), rounded_mask(n, radius))
    return out


def render(size):
    return draw_icon(size * SS).resize((size, size), Image.LANCZOS)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview", action="store_true")
    args = ap.parse_args()

    sizes = {"mipmap-mdpi": 48, "mipmap-hdpi": 72, "mipmap-xhdpi": 96,
             "mipmap-xxhdpi": 144, "mipmap-xxxhdpi": 192}
    if args.preview:
        outdir = os.path.join(APP_DIR, "build", "icon_preview")
        os.makedirs(outdir, exist_ok=True)
        for sz in (512, 192, 144, 96, 72, 48):
            p = os.path.join(outdir, "icon_%d.png" % sz)
            render(sz).save(p)
            print(p)
        return

    for k, sz in sizes.items():
        d = os.path.join(APP_DIR, "app", "res", k)
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, "ic_launcher.png")
        render(sz).save(p)
        print("%s  %dx%d" % (p, sz, sz))


if __name__ == "__main__":
    main()
