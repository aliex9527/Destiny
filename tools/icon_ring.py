# -*- coding: utf-8 -*-
"""球环版图标：5 红 + 2 蓝小球围成一圈，中间「运」字。

用法：
    python tools/icon_ring.py                          # 出预览图（浅底）
    python tools/icon_ring.py --variant dark           # 出预览图（深底）
    python tools/icon_ring.py --apply                  # 写入 app/res/mipmap-*
"""
import os
import math
import argparse
from PIL import Image, ImageDraw, ImageFilter
import make_icon as M

MARK = "\u8fd0"  # 运
RED = ((255, 168, 146), (216, 38, 26), (134, 8, 4))
BLUE = ((158, 208, 255), (42, 118, 246), (8, 44, 144))

# 几何参数（相对边长的比例）
BALL_R = 0.092      # 球半径
RING_R = 0.292      # 环半径（球心到中心）
MARK_PX = 0.228     # 「运」字号
START_DEG = -90.0   # 首个球的角度（-90 = 正上方）
BLUE_IDX = (3, 4)   # 哪两颗是后区蓝球


def _vignette(n, strength=46):
    """四角压暗，给纯色底一点纵深。"""
    g = Image.radial_gradient("L").resize((n, n), Image.BILINEAR)
    a = g.point(lambda v: max(0, int((v - 128) * strength / 127.0)))
    layer = Image.new("RGBA", (n, n), (120, 60, 48, 255))
    layer.putalpha(a)
    return layer


def _bg(n, variant):
    if variant == "light":
        img = M.linear_gradient(n, (255, 255, 255), (248, 239, 235), 118).convert("RGBA")
        # 环内暖光，让球环有"聚拢"的视觉重心
        img = Image.alpha_composite(img, M.radial_glow(n, n * 0.50, n * 0.52, n * 0.55,
                                                      (255, 196, 176), 40))
        img = Image.alpha_composite(img, M.radial_glow(n, n * 0.18, n * 0.08, n * 0.80,
                                                      (255, 255, 255), 80))
        img = Image.alpha_composite(img, _vignette(n, 30))
        return img, (194, 20, 14, 255), (140, 46, 34), 108
    img = M.linear_gradient(n, (42, 38, 50), (14, 12, 18), 128).convert("RGBA")
    img = Image.alpha_composite(img, M.radial_glow(n, n * 0.48, n * 0.46, n * 0.62,
                                                  (255, 108, 78), 58))
    return img, (255, 255, 255, 255), (0, 0, 0), 150


def draw_ring(n, variant="light"):
    img, mark_color, sh_color, sh_alpha = _bg(n, variant)

    r = n * BALL_R
    rr = n * RING_R
    cx, cy = n * 0.5, n * 0.508

    spots = []
    for i in range(7):
        a = math.radians(START_DEG + i * 360 / 7)
        spots.append((cx + rr * math.cos(a), cy + rr * math.sin(a),
                      "blue" if i in BLUE_IDX else "red"))

    # 球体精灵只画一次（5 红同一张、2 蓝同一张）
    rs = int(round(r * 2))
    sprites = {
        "red": M.sphere(rs, *RED, rim_color=(255, 246, 242, 96), rim_w=0.022, gloss_alpha=118),
        "blue": M.sphere(rs, *BLUE, rim_color=(255, 255, 255, 175), rim_w=0.022, gloss_alpha=118),
    }

    # 统一落影（右下偏移，模拟左上光源）
    sh = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ds = ImageDraw.Draw(sh)
    for bx, by, _ in spots:
        ds.ellipse([bx - r * 0.94 + r * 0.10, by - r * 0.90 + r * 0.12,
                    bx + r * 1.02 + r * 0.10, by + r * 1.14 + r * 0.12],
                   fill=sh_color + (sh_alpha,))
    img = Image.alpha_composite(img, sh.filter(ImageFilter.GaussianBlur(n * 0.013)))

    for bx, by, kind in spots:
        img.alpha_composite(sprites[kind], (int(round(bx - r)), int(round(by - r))))

    img = M.draw_glyph(img, n, MARK, cx, cy + n * 0.004, n * MARK_PX, mark_color)

    mask = M.rounded_mask(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)

    edge = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(edge).rounded_rectangle(
        [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
        outline=(255, 255, 255, 165 if variant == "light" else 34),
        width=max(1, int(n * 0.010)))
    edge = edge.filter(ImageFilter.GaussianBlur(n * 0.003))
    edge.putalpha(Image.composite(edge.split()[3], Image.new("L", (n, n), 0), mask))
    return Image.alpha_composite(out, edge)


def render(size, variant="light"):
    ss = 4 if size <= 200 else 2
    return draw_ring(size * ss, variant).resize((size, size), Image.LANCZOS)


def main():
    global BALL_R, RING_R, MARK_PX, START_DEG, BLUE_IDX
    ap = argparse.ArgumentParser()
    ap.add_argument("--variant", default="light")
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--ball-r", type=float, default=BALL_R)
    ap.add_argument("--ring-r", type=float, default=RING_R)
    ap.add_argument("--mark-px", type=float, default=MARK_PX)
    ap.add_argument("--start", type=float, default=START_DEG)
    ap.add_argument("--tag", default="")
    ap.add_argument("--blue", default=",".join(str(i) for i in BLUE_IDX))
    args = ap.parse_args()

    BALL_R, RING_R, MARK_PX, START_DEG = args.ball_r, args.ring_r, args.mark_px, args.start
    BLUE_IDX = tuple(int(v) for v in args.blue.split(","))

    if args.apply:
        for k, sz in (("mipmap-mdpi", 48), ("mipmap-hdpi", 72), ("mipmap-xhdpi", 96),
                      ("mipmap-xxhdpi", 144), ("mipmap-xxxhdpi", 192)):
            p = os.path.join(M.APP_DIR, "app", "res", k, "ic_launcher.png")
            render(sz, args.variant).save(p)
            print("%s  %dx%d" % (p, sz, sz))
        return

    out = os.path.join(M.APP_DIR, "build", "icon_preview")
    os.makedirs(out, exist_ok=True)
    for sz in (384, 192, 96, 48):
        p = os.path.join(out, "ring_%s%s_%d.png" % (args.variant, args.tag, sz))
        render(sz, args.variant).save(p)
        print(p)


if __name__ == "__main__":
    main()
