#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""把系统中文字体裁剪成 APK 内嵌的子集字体。

egui 不绑定任何字体，中文必须自带。完整字体 10~17MB，直接塞进 APK 无法接受，
因此按实际用到的字符集裁剪。字符集取三部分并集：

  1. 前端与 Rust 源码里出现的全部字符（自动扫描，改文案后重跑即可）
  2. GB2312 全集 6763 字（保证界面外的新文案、开奖公告、错误提示不缺字）
  3. ASCII 与常用标点、数学符号

用法：
    python tools/make_font.py                      # 默认用系统仿宋
    python tools/make_font.py --src "D:/字体/FZFSJW.TTF"
    python tools/make_font.py --src X.ttf --no-gb2312   # 只保留实际用字，体积最小
"""
import argparse
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, 'rust', 'assets')
OUT_NAME = 'font_subset.ttf'

# 默认字体：Windows 自带仿宋。方正仿宋_GB2312 需自行指定 --src。
DEFAULT_SRC = r'C:\Windows\Fonts\simfang.ttf'

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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--src', default=DEFAULT_SRC, help='源字体文件')
    ap.add_argument('--out', default=os.path.join(OUT_DIR, OUT_NAME), help='输出路径')
    ap.add_argument('--no-gb2312', action='store_true', help='不并入 GB2312 全集')
    ap.add_argument('--keep', action='store_true', help='保留全部字形（仅调试用）')
    a = ap.parse_args()

    src = a.src
    if not os.path.exists(src):
        print('[失败] 找不到源字体：%s' % src, file=sys.stderr)
        print('       系统仿宋一般在 C:\\Windows\\Fonts\\simfang.ttf', file=sys.stderr)
        print('       方正仿宋_GB2312 请用 --src 指定实际路径', file=sys.stderr)
        return 1

    chars = scan_chars()
    n_src = len(chars)
    if not a.no_gb2312:
        chars |= gb2312_chars()
    chars |= set(EXTRA)
    chars |= set(chr(c) for c in range(0x20, 0x7F))  # ASCII 可见字符
    chars.discard('\n')

    text = ''.join(sorted(chars))
    print('[font] 源码用字 %d 个，子集合计 %d 个字符' % (n_src, len(text)))
    print('[font] 源字体 %.1f MB  %s' % (os.path.getsize(src) / 1048576.0, src))

    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    tmp = a.out + '.text'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)

    cmd = [sys.executable, '-m', 'fontTools.subset', src,
           '--text-file=' + tmp, '--output-file=' + a.out,
           '--layout-features=*', '--no-hinting', '--desubroutinize',
           '--drop-tables+=DSIG', '--name-IDs=*', '--recalc-bounds']
    if a.keep:
        cmd = [sys.executable, '-m', 'fontTools.subset', src, '--output-file=' + a.out,
               '--unicodes=*']
    p = subprocess.run(cmd, capture_output=True)
    os.remove(tmp)
    if p.returncode != 0:
        print('[失败] 子集化出错：\n%s' % (p.stderr or b'').decode('utf-8', 'replace'), file=sys.stderr)
        return 1

    size = os.path.getsize(a.out)
    ratio = size * 100.0 / os.path.getsize(src)
    print('[font] 输出 %s  %s 字节（原始 %.1f%%）'
          % (os.path.relpath(a.out, ROOT), format(size, ','), ratio))
    return 0


if __name__ == '__main__':
    sys.exit(main())
