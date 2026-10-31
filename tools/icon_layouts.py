# -*- coding: utf-8 -*-
"""图标构图实验：球环位置、字号、字位置全部自由摆放。

复用 icon_design.py 的球体着色、背景、文字渲染，只重写"组装"这一段，
这样几套配色（A/B/C）可以直接套到任意构图。

用法：
    python tools/icon_layouts.py --sizes 512,192,96,48
    python tools/icon_layouts.py --only L1,L4
"""
import os
import sys
import math
import argparse
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import icon_design as D  # noqa: E402

OUT = os.path.join(D.APP_DIR, "build", "icon_preview")

# 构图参数说明
#   ring  球环圆心的 (x, y)，可以超出 0~1 让环被画布裁掉
#   rr    环半径（球心到环心）
#   ball  球的半径
#   squash 环的纵向压缩，1.0 = 正圆，<1 压成椭圆
#   start 第一颗球的角度（-90 = 正上方）
#   blue  哪两颗是后区蓝球
#   mark  「运」字的中心 (x, y)
#   mark_px 字号的相对大小（1.0 = 整个边长）
#   front 字是否压在球上面
LAYOUTS = {
    "L1": dict(name="上环下字", ring=(0.500, 0.315), rr=0.202, ball=0.064,
               mark=(0.500, 0.744), mark_px=0.272, front=True,
               halo=((255, 246, 236), 40, 0.230)),
    "L2": dict(name="左环右字", ring=(0.325, 0.540), rr=0.158, ball=0.051,
               mark=(0.738, 0.540), mark_px=0.285, front=True,
               halo=((255, 246, 236), 34, 0.190)),
    "L3": dict(name="满环套字", ring=(0.500, 0.502), rr=0.350, ball=0.100,
               mark=(0.500, 0.502), mark_px=0.300, front=True,
               halo=((255, 246, 236), 56, 0.235)),
    "L4": dict(name="环下字上", ring=(0.500, 0.665), rr=0.202, ball=0.064,
               mark=(0.500, 0.238), mark_px=0.272, front=True,
               halo=((255, 246, 236), 38, 0.225)),
    "L5": dict(name="右环左字", ring=(0.675, 0.540), rr=0.158, ball=0.051,
               mark=(0.262, 0.540), mark_px=0.285, front=True,
               halo=((255, 246, 236), 34, 0.190)),
    "L6": dict(name="倾斜环", ring=(0.500, 0.278), rr=0.212, ball=0.057,
               start=-68.0, blue=(3, 4),
               mark=(0.500, 0.740), mark_px=0.285, front=True,
               halo=((255, 246, 236), 38, 0.225)),
}


def _spots(n, L):
    rcx, rcy = L["ring"]
    rr = L["rr"]
    sq = L.get("squash", 1.0)
    cnt = L.get("count", 7)
    start = L.get("start", -90.0)
    blue = L.get("blue", (3, 4))
    out = []
    for i in range(cnt):
        a = math.radians(start + i * 360.0 / cnt)
        out.append((n * (rcx + rr * math.cos(a)),
                    n * (rcy + rr * sq * math.sin(a)),
                    "blue" if i in blue else "red"))
    return out


def draw_layout(n, L, v="A"):
    P = D.VARIANTS[v]
    img = D._bg(n, v)

    for g in P["glows"]:
        img = Image.alpha_composite(img, D._glow(n, g[0], g[1], g[2], g[3], g[4]))
    if P.get("sheen"):
        img = Image.alpha_composite(img, D._sheen(n, P["sheen"]))
    if P.get("frame"):
        f = P["frame"]
        img = Image.alpha_composite(img, D._frame(n, f[0], f[1], f[2], f[3], f[4]))

    r = n * L["ball"]
    spots = _spots(n, L)
    mx, my = L["mark"][0], L["mark"][1]

    if L.get("halo"):
        hc, ha, hr = L["halo"]
        img = Image.alpha_composite(img, D._glow(n, mx, my, hr, hc, ha, 1.4))

    # 球体的落影（统一左上光 → 右下偏移）
    sh = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ds = ImageDraw.Draw(sh)
    for bx, by, _ in spots:
        ox, oy = r * 0.13, r * 0.17
        ds.ellipse([bx - r * 0.94 + ox, by - r * 0.90 + oy,
                    bx + r * 0.96 + ox, by + r * 1.10 + oy],
                   fill=tuple(P["shade"][0]) + (P["shade"][1],))
    img = Image.alpha_composite(img, sh.filter(ImageFilter.GaussianBlur(n * 0.011)))

    txt = D._text(n, D.MARK, n * mx, n * my, n * L["mark_px"],
                  color=P["mark_color"], grad=P["mark_grad"], shadow=P["mark_shadow"])

    if not L.get("front", True):
        img = Image.alpha_composite(img, txt)

    sprites = {}
    for kind in ("red", "blue"):
        light, base, bounce = P[kind]
        sprites[kind] = D._ball(max(4, int(round(r * 2))), light, base, bounce, P["rim_glow"])

    ball_layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    for bx, by, kind in spots:
        s = sprites[kind]
        ball_layer.paste(s, (int(round(bx - r)), int(round(by - r))), s)
    img = Image.alpha_composite(img, ball_layer)

    if L.get("front", True):
        img = Image.alpha_composite(img, txt)

    # 圆角裁切 + 边缘描边
    mask = D._rounded(n, n * 0.228)
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)

    edge = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(edge).rounded_rectangle(
        [n * 0.004, n * 0.004, n * 0.996, n * 0.996], radius=n * 0.226,
        outline=tuple(P["edge"][:3]) + (P["edge"][3],), width=max(1, int(n * 0.009)))
    edge = edge.filter(ImageFilter.GaussianBlur(n * 0.003))
    edge.putalpha(Image.composite(edge.split()[3], Image.new("L", (n, n), 0), mask))
    return Image.alpha_composite(out, edge)


def render(size, L, v="A"):
    ss = 4 if size <= 256 else 2
    return draw_layout(size * ss, L, v).resize((size, size), Image.LANCZOS)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sizes", default="512,192,96,48")
    ap.add_argument("--variant", default="A")
    ap.add_argument("--only", default="")
    ap.add_argument("--tag", default="")
    args = ap.parse_args()

    os.makedirs(OUT, exist_ok=True)
    keys = [k.strip() for k in args.only.split(",") if k.strip()] or list(LAYOUTS)
    sizes = [int(s) for s in args.sizes.split(",")]
    for k in keys:
        L = LAYOUTS[k]
        for sz in sizes:
            p = os.path.join(OUT, "%s%s_%d.png" % (k, args.tag, sz))
            render(sz, L, args.variant).save(p)
        print("%s %s ok" % (k, L["name"]))


if __name__ == "__main__":
    main()
