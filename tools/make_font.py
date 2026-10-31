#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""把系统中文字体裁剪成 APK 内嵌的子集字体（常规 + 粗体两个字重）。

egui 不绑定任何字体，中文必须自带；而且 egui 不支持合成加粗，
只有 Embed 一份真正粗体，`font-weight:700` 的标题才不会糊成一片。

输出两个文件：
  rust/assets/font_regular.ttf   常规字重，覆盖 GB2312 全集，保证新文案不缺字
  rust/assets/font_bold.ttf      粗体，只覆盖源码实际用字（体积小），
                                 缺字时 egui 自动回退到常规字重，不会出豆腐块

用法：
    python tools/make_font.py                              # 默认微软雅黑（最接近原版 WebView）
    python tools/make_font.py --regular X.ttf --bold Y.ttf  # 指定任意两个字重文件
    python tools/make_font.py --regular simfang.ttf --bold simfang.ttf   # 仿宋（单字重会退化）
"""
import argparse
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, 'rust', 'assets')

# 默认字体：微软雅黑。与原版 CSS 的 "Microsoft YaHei" 同一字体，
# 也是 WebView 在 Windows/多数国产 ROM 上的实际回退目标，观感最接近。
DEF_REGULAR = r'C:\Windows\Fonts\msyh.ttc'
DEF_BOLD = r'C:\Windows\Fonts\msyhbd.ttc'

# 扫描范围：这些目录里的文本决定了「实际用到的字符」
SCAN_DIRS = ['app/src/main/assets', 'rust/src']
SCAN_EXT = ('.rs', '.html', '.js', '.css', '.toml', '.xml')

# 常用符号：中文标点、全角符号、数学与箭头，界面上可能会用到
EXTRA = (
    '—…‘’“”·、。，；：？！（）《》〈〉【】「」『』〔〕％‰℃€$¥'
    '≤≥≠≈±×÷√∞°′″←→↑↓✓✗●○◆■□▲▼★☆'
)


def scan_chars():
    """收集源码中实际出现的字符，避免裁掉正在使用的字。"""
    chars = set()
    for d in SCAN_DIRS:
        base = os.path.join(ROOT, d)
        if not os.path.isdir(base):
            continue
        for dirpath, _, files in os.walk(base):
            for fn in files:
                if not fn.endswith(SCAN_EXT):
                    continue
                try:
                    with open(os.path.join(dirpath, fn), encoding='utf-8') as f:
                        chars.update(f.read())
                except (UnicodeDecodeError, OSError):
                    pass
    return chars


def gb2312_chars():
    """GB2312 可表示的全部字符，覆盖 6763 个汉字与全角符号。"""
    out = set()
    for cp in range(0x4E00, 0x9FA6):
        ch = chr(cp)
        try:
            ch.encode('gb2312')
        except UnicodeEncodeError:
            continue
        out.add(ch)
    for cp in list(range(0x3000, 0x3040)) + list(range(0xFF00, 0xFFF0)):
        ch = chr(cp)
        try:
            ch.encode('gb2312')
        except UnicodeEncodeError:
            continue
        out.add(ch)
    return out


def split_face(src):
    """'C:/x/msyh.ttc#1' -> ('C:/x/msyh.ttc', 1)。用于从 TTC 里挑具体 face。"""
    if '#' in src:
        path, _, n = src.rpartition('#')
        return path, int(n)
    return src, 0


def build(src, chars, out):
    path, face = split_face(src)
    if not os.path.exists(path):
        print('[失败] 找不到源字体：%s' % path, file=sys.stderr)
        return False

    text = ''.join(sorted(chars))
    tmp = out + '.text'
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)

    cmd = [sys.executable, '-m', 'fontTools.subset', path,
           '--font-number=%d' % face,
           '--text-file=' + tmp, '--output-file=' + out,
           '--layout-features=*', '--no-hinting', '--desubroutinize',
           '--drop-tables+=DSIG', '--name-IDs=*', '--recalc-bounds']
    p = subprocess.run(cmd, capture_output=True)
    os.remove(tmp)
    if p.returncode != 0:
        print('[失败] 子集化出错（%s）：\n%s'
              % (path, (p.stderr or b'').decode('utf-8', 'replace')), file=sys.stderr)
        return False

    print('[font] %-18s %-34s %8d 字符  %6.0f KB  (%.1f%%)'
          % (os.path.basename(out), os.path.basename(path),
             len(chars), os.path.getsize(out) / 1024.0,
             os.path.getsize(out) * 100.0 / os.path.getsize(path)))
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--regular', default=DEF_REGULAR, help='常规字重源文件（可写 path#face）')
    ap.add_argument('--bold', default=DEF_BOLD, help='粗体源文件（可写 path#face）')
    ap.add_argument('--no-gb2312', action='store_true', help='常规字重也不并入 GB2312 全集')
    ap.add_argument('--out-dir', default=OUT_DIR)
    a = ap.parse_args()

    used = scan_chars()
    print('[font] 源码实际用字 %d 个' % len(used))

    # 常规：源码用字 + GB2312 全集 + 符号 + ASCII
    reg = set(used) | set(EXTRA) | set(chr(c) for c in range(0x20, 0x7F))
    if not a.no_gb2312:
        reg |= gb2312_chars()

    # 粗体：只覆盖实际用字，体积能省一大半；缺字由 egui 回退到常规字重
    bold = set(used) | set(EXTRA) | set(chr(c) for c in range(0x20, 0x7F))

    ok = build(a.regular, reg, os.path.join(a.out_dir, 'font_regular.ttf'))
    ok = build(a.bold, bold, os.path.join(a.out_dir, 'font_bold.ttf')) and ok

    if ok:
        total = sum(os.path.getsize(os.path.join(a.out_dir, n))
                    for n in ('font_regular.ttf', 'font_bold.ttf'))
        print('[font] 合计 %.2f MB' % (total / 1048576.0))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
