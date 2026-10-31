#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""把前端用的 data.js 转成 Rust 版内嵌的纯文本数据。

来源：app/src/main/assets/data.js 里的 window.DLT_RAW="期号,前5,后2,日期;..."
输出：rust/assets/draws.txt，每期一行

之所以不复用 data.js：Rust 版没有 JS 运行时，直接解析一行期数据的成本
比解析 JS 字符串字面量低，也让数据文件可以用文本工具抽查。
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, 'app', 'src', 'main', 'assets', 'data.js')
OUT = os.path.join(ROOT, 'rust', 'assets', 'draws.txt')


def main():
    text = open(SRC, encoding='utf-8').read()
    m = re.search(r'window\.DLT_RAW\s*=\s*"([^"]*)"', text)
    if not m:
        print('[失败] 没找到 window.DLT_RAW', file=sys.stderr)
        return 1
    raw = m.group(1)
    lines = []
    for rec in raw.split(';'):
        rec = rec.strip()
        if not rec:
            continue
        parts = rec.split(',')
        if len(parts) != 9:
            print('[跳过] 字段数异常：%s' % rec, file=sys.stderr)
            continue
        lines.append(rec)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(lines) + '\n')
    size = os.path.getsize(OUT)
    print('[data] %d 期  %s 字节' % (len(lines), format(size, ',')))
    print('[data] 首期 %s  末期 %s' % (lines[0].split(',')[0], lines[-1].split(',')[0]))
    print('[data] 输出 %s' % os.path.relpath(OUT, ROOT))
    return 0


if __name__ == '__main__':
    sys.exit(main())
