#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
本地一键发布：构建签名 APK -> 推到 release-assets 分支 -> 打 tag 并推送。

为什么这样做：签名密钥只保存在本机，绝不进仓库，所以 CI 无法构建。
CI 只负责「分发」：收到 v* tag 后，从 release-assets 分支取回本脚本推上去的
APK，核对包内版本号与 tag 一致，然后创建 Release 并附上安装包。

release-assets 是单提交孤儿分支，每次强制覆盖，不累积历史，也不影响 main。

用法：
    python tools/release.py                     # 用 tools/build_apk.py 里的版本号
    python tools/release.py --version 1.5.1     # 指定版本号（同时决定 versionCode）
    python tools/release.py --no-build          # 复用 dist/ 里已构建好的包
    python tools/release.py --retag             # 允许覆盖已存在的同名 tag
    python tools/release.py --no-push           # 只构建、只打本地 tag，不联网

发布完成后约 30 秒可在 Releases 页看到结果；若失败，日志会被 CI 推到
ci-logs 分支，`git fetch origin ci-logs` 后看 .ci/ci-build.log。
"""
import argparse
import hashlib
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, 'dist')
KS = os.path.join(ROOT, 'keystore', 'dlt-release.jks')
APP_NAME = 'Destiny'
BRANCH = 'release-assets'


def log(msg):
    print('[release] %s' % msg)


def fail(msg, code=1):
    print('[失败] %s' % msg, file=sys.stderr)
    sys.exit(code)


def git(*args, stdin=None):
    # 必须走二进制 stdin：Windows 下 text 模式会把 \n 转成 \r\n，
    # 导致 mktree 生成 "dist\r/xxx.apk\r" 这种带回车符的路径，CI 匹配不到。
    data = stdin.encode('utf-8') if isinstance(stdin, str) else stdin
    p = subprocess.run(['git'] + list(args), cwd=ROOT,
                       input=data, capture_output=True)
    out = (p.stdout or b'').decode('utf-8', 'replace')
    err = (p.stderr or b'').decode('utf-8', 'replace')
    if p.returncode != 0:
        fail('git %s 失败：%s' % (' '.join(args), (err or out).strip()))
    return out.strip()


def base_version():
    """从 build_apk.py 读取默认版本号，保持单一来源。"""
    src = open(os.path.join(ROOT, 'tools', 'build_apk.py'), encoding='utf-8').read()
    m = re.search(r"^VERSION = _ver or '([^']+)'", src, re.M)
    return m.group(1) if m else None


def version_code(ver):
    p = ([int(x) for x in re.findall(r'\d+', ver)[:3]] + [0, 0, 0])[:3]
    return max(1, p[0] * 10000 + p[1] * 100 + p[2])


def repo_slug():
    """从 origin 地址推导 owner/repo，仓库改名后不用改代码。"""
    url = git('remote', 'get-url', 'origin')
    m = re.search(r'github\.com[:/]+([^/]+)/([^/\s]+?)(?:\.git)?$', url)
    return '%s/%s' % (m.group(1), m.group(2)) if m else url


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def build(ver):
    if not os.path.exists(KS):
        fail('缺少签名密钥 %s\n       密钥只在本机保存，构建必须有它；'
             '新机器请从备份恢复，不要重新生成（会导致无法覆盖升级）' % KS)
    log('版本 %s（versionCode %d）' % (ver, version_code(ver)))
    log('开始构建，约 10~20 秒……')
    env = dict(os.environ, DLT_VERSION=ver, DLT_REQUIRE_KS='1')
    p = subprocess.run([sys.executable, os.path.join('tools', 'build_apk.py')],
                       cwd=ROOT, env=env)
    if p.returncode != 0:
        fail('构建失败，退出码 %d' % p.returncode)


def find_apk(ver):
    path = os.path.join(DIST, '%s_v%s.apk' % (APP_NAME, ver))
    if not os.path.exists(path):
        cands = [f for f in os.listdir(DIST) if f.lower().endswith('.apk')] if os.path.isdir(DIST) else []
        fail('找不到 %s\n       dist/ 现有：%s' % (path, ', '.join(cands) or '(空)'))
    return path


def push_assets(apk):
    """把 APK 写成一条孤儿提交推到 release-assets 分支，不动工作区与索引。"""
    name = os.path.basename(apk)
    blob = git('hash-object', '-w', apk)
    tree_dist = git('mktree', stdin='100644 blob %s\t%s\n' % (blob, name))
    tree_root = git('mktree', stdin='040000 tree %s\tdist\n' % tree_dist)
    commit = git('commit-tree', tree_root, '-m', 'release %s' % name)
    expected = 'dist/' + name
    listing = git('ls-tree', '-r', '--name-only', commit)
    if listing.splitlines() != [expected]:
        fail('release-assets 提交内容异常，期望只有 %r，实际：%r' % (expected, listing))
    git('push', '-f', 'origin', '%s:refs/heads/%s' % (commit, BRANCH))
    log('已推送 %s -> 分支 %s（commit %s）' % (name, BRANCH, commit[:8]))
    return commit


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--version', help='版本号，如 1.5.1（默认取 build_apk.py 的 VERSION）')
    ap.add_argument('--no-build', action='store_true', help='复用 dist/ 里已构建好的包')
    ap.add_argument('--retag', action='store_true', help='允许覆盖已存在的同名 tag')
    ap.add_argument('--no-push', action='store_true', help='只构建与打本地 tag，不推送')
    a = ap.parse_args()

    ver = a.version or base_version()
    if not ver:
        fail('无法确定版本号，请用 --version 指定')
    tag = 'v' + ver

    dirty = git('status', '--porcelain', '--', 'app', 'tools', '.github')
    if dirty:
        log('提示：app/ tools/ .github/ 有未提交改动，tag 指向的提交将不含这些改动')

    if not a.no_build:
        build(ver)
    apk = find_apk(ver)
    size = os.path.getsize(apk)
    digest = sha256(apk)
    log('产物 %s  %s 字节' % (os.path.relpath(apk, ROOT), format(size, ',')))
    log('sha256 %s' % digest)

    if not a.no_push:
        push_assets(apk)

    # tag
    exists = subprocess.run(['git', 'rev-parse', '-q', '--verify', 'refs/tags/' + tag],
                            cwd=ROOT, capture_output=True).returncode == 0
    if exists and not a.retag:
        fail('tag %s 已存在；要重新发布请加 --retag' % tag)
    args = ['tag', '-f', '-a', tag, '-m', '%s %s：本地构建签名发布' % (APP_NAME, ver)] if exists \
        else ['tag', '-a', tag, '-m', '%s %s：本地构建签名发布' % (APP_NAME, ver)]
    git(*args)
    log('本地 tag %s 已创建' % tag)

    if a.no_push:
        log('--no-push：未推送。手动执行 git push -f origin %s 可触发发布' % tag)
        return

    # 强制更新已有的 tag 不一定会触发 CI，实测需先删远端 tag 再推（等价于新建）
    if git('ls-remote', '--tags', 'origin', 'refs/tags/' + tag):
        log('远端已有同名 tag，先删除再推，确保触发发布')
        git('push', 'origin', ':refs/tags/' + tag)
    git('push', 'origin', 'refs/tags/' + tag)
    log('已推送 tag %s，CI 开始发布' % tag)
    slug = repo_slug()
    print()
    print('结果页：https://github.com/%s/releases/tag/%s' % (slug, tag))
    print('日志：  https://github.com/%s/actions' % slug)
    print('失败排查：git fetch origin ci-logs && git show origin/ci-logs:.ci/ci-build.log')


if __name__ == '__main__':
    main()
