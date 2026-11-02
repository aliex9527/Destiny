#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""给单字重字体（如仿宋）合成一个粗体字重。

仿宋只有 Regular 一个字重，而 egui 不会合成加粗——直接把 Regular 当粗体用，
标题和正文就完全一样，界面层级全丢了。

做法：沿每个字形的轮廓做描边（stroke），再与原轮廓求并集。等价于把笔画
整体向外膨胀 width/2，观感接近真粗体，而不是"描边空心"的效果。

用法：
    python tools/embolden_font.py --src in.ttf --dst out.ttf --width 6
    python tools/embolden_font.py --src in.ttf --dst out.ttf --width 6 --preview 推演
"""
import argparse
import os
import sys

import pathops
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


def load_path(glyph_set, name):
    """取字形的轮廓；空字形返回 None。"""
    p = pathops.Path()
    try:
        glyph_set[name].draw(p.getPen())
    except Exception:
        return None
    return p if len(p.verbs) else None


def embolden_path(p, width, join='miter', miter=2.0):
    """描边后与原轮廓合并，把笔画整体撑宽。"""
    cap = pathops.LineCap.BUTT_CAP
    if join == 'round':
        j, m = pathops.LineJoin.ROUND_JOIN, 4.0
    else:
        j, m = pathops.LineJoin.MITER_JOIN, miter

    p.simplify(fix_winding=True, keep_starting_points=False)

    # stroke / simplify 都是原地操作，先留一份原始轮廓
    solid = pathops.Path()
    p.draw(solid.getPen())
    stroked = pathops.Path()
    p.draw(stroked.getPen())
    stroked.stroke(width, cap, j, m)
    if not len(stroked.verbs):
        return p
    return pathops.op(solid, stroked, pathops.PathOp.UNION, fix_winding=True)


def build(src, dst, width, join, miter, preview=None):
    font = TTFont(src)
    glyph_set = font.getGlyphSet()
    glyf = font['glyf']
    order = font.getGlyphOrder()
    hinting = font['maxp'].maxSizeOfInstructions > 0

    upem = font['head'].unitsPerEm
    print('[embolden] %s  upem=%d  字形 %d  宽度 %.1f (%.1f%% em)'
          % (os.path.basename(src), upem, len(order), width, width * 100.0 / upem))

    out = {}
    n = 0
    for name in order:
        p = load_path(glyph_set, name)
        if p is None:
            continue
        q = embolden_path(p, width, join, miter)
        pen = TTGlyphPen(None)
        q.draw(Cu2QuPen(pen, max_err=0.4, reverse_direction=False))
        g = pen.glyph()
        g.recalcBounds(glyf)
        out[name] = g
        n += 1
        if preview and name in preview:
            print('  [%s] 轮廓点 %d -> %d  面积 %.0f -> %.0f'
                  % (name, len(p.verbs), len(q.verbs), p.area, q.area))

    for name, g in out.items():
        glyf[name] = g

    # 合成为粗体，避免系统/渲染器把它当常规字重缓存
    font['OS/2'].usWeightClass = 700
    os2 = font['OS/2']
    if hasattr(os2, 'fsSelection'):
        os2.fsSelection = (os2.fsSelection | 0x20) & ~0x40
    if 'head' in font:
        font['head'].macStyle |= 0x1
    # 加粗后旧 hinting 全失效，留着只会画歪
    if hinting and 'fpgm' in font:
        for t in ('fpgm', 'prep', 'cvt '):
            if t in font:
                del font[t]
    for name_rec in font['name'].names:
        if name_rec.nameID in (1, 2, 4, 6, 16, 17):
            try:
                cur = name_rec.toUnicode()
            except Exception:
                continue
            if name_rec.nameID == 2:
                name_rec.string = 'Bold'
            elif name_rec.nameID == 6:
                name_rec.string = cur.replace('Regular', 'Bold').replace('regular', 'Bold')
            elif name_rec.nameID in (4,):
                name_rec.string = cur
    font.save(dst)
    print('[embolden] %-18s %d 字形  %6.0f KB'
          % (os.path.basename(dst), n, os.path.getsize(dst) / 1024.0))
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--src', required=True)
    ap.add_argument('--dst', required=True)
    ap.add_argument('--width', type=float, default=6.0,
                    help='描边总宽（字体单位）。膨胀量为一半，upem=256 时 5~7 合适')
    ap.add_argument('--join', default='miter', choices=('miter', 'round'))
    ap.add_argument('--miter', type=float, default=2.0)
    ap.add_argument('--preview', default=None, help='打印指定字符的轮廓变化')
    a = ap.parse_args()

    if not os.path.exists(a.src):
        print('[失败] 找不到 %s' % a.src, file=sys.stderr)
        return 1
    return 0 if build(a.src, a.dst, a.width, a.join, a.miter, a.preview) else 1


if __name__ == '__main__':
    sys.exit(main())
