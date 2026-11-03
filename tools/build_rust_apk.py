#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Destiny Rust 版 APK 构建（无 Gradle、无 Java 源码、无 dex）

cargo build --target aarch64-linux-android
    -> aapt2 compile / link（复用 app/res 图标与主题）
    -> 塞入 lib/<abi>/libdestiny.so
    -> zipalign -> apksigner

与 tools/build_apk.py 的关键差异：
  * 没有 javac / d8 / classes.dex，清单里 android:hasCode="false"
  * 入口是 android.app.NativeActivity，加载 libdestiny.so 里的 android_main
  * 中文字体已 include_bytes! 进 .so，不需要 assets

aapt2 在中文路径下会报 "failed to open directory"，所以中间产物一律在 ASCII 临时目录完成。

用法：
    python tools/build_rust_apk.py                    # arm64，版本取脚本默认
    python tools/build_rust_apk.py --version 2.0.1
    python tools/build_rust_apk.py --abi both         # arm64 + armv7
    python tools/build_rust_apk.py --no-build         # 复用已有 .so，只打包
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOL = os.environ.get('DLT_TOOLCHAIN') or os.path.join(ROOT, 'toolchain')
JDK = os.environ.get('DLT_JDK') or os.path.join(TOOL, 'zulu17.54.21-ca-jdk17.0.13-win_x64')
BT = os.environ.get('DLT_BT') or os.path.join(TOOL, 'android-sdk', 'build-tools', '34.0.0')
ANDROID_JAR = os.environ.get('DLT_ANDROID_JAR') or os.path.join(TOOL, 'android.jar')
RUST_DIR = os.path.join(ROOT, 'rust')
RES_SRC = os.path.join(ROOT, 'app', 'res')
DIST = os.path.join(ROOT, 'dist')
KS_SRC = os.environ.get('DLT_KS') or os.path.join(ROOT, 'keystore', 'dlt-release.jks')

KS_PASS = os.environ.get('DLT_KS_PASS') or 'micai2026'
KS_ALIAS = os.environ.get('DLT_KS_ALIAS') or 'dlt'
APP_NAME = 'Destiny'
LIB_NAME = 'destiny'          # 清单里 android.app.lib_name，实际文件是 libdestiny.so

ABIS = {
    'arm64': ('aarch64-linux-android', 'arm64-v8a'),
    'armv7': ('armv7-linux-androideabi', 'armeabi-v7a'),
}

STAGE = os.path.join(os.environ.get('TEMP') or tempfile.gettempdir(), 'dltrust')

# 与 rust/.cargo/config.toml 的 [build] target-dir 保持一致。
# 放在 ASCII 路径是硬性要求：GNU ld 处理不了中文目录。
TARGET_DIR = os.environ.get('DLT_RUST_TARGET_DIR') or 'C:/rsbuild/destiny'


def _pick(bindir, name):
    for cand in (name + '.exe', name):
        p = os.path.join(bindir, cand)
        if os.path.exists(p):
            return p
    return name


JAVA = _pick(os.path.join(JDK, 'bin'), 'java')
AAPT2 = _pick(BT, 'aapt2')
ZIPALIGN = _pick(BT, 'zipalign')
SIGNER_JAR = os.path.join(BT, 'lib', 'apksigner.jar')


def run(cmd, desc, cwd=None):
    print('\n>> %s' % desc)
    p = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    out = p.stdout.decode('utf-8', 'ignore').strip()
    if out:
        print(out[-3000:])
    if p.returncode != 0:
        print('\n[失败] %s  (exit %d)' % (desc, p.returncode))
        sys.exit(1)
    return out


def version_code(ver):
    p = ([int(x) for x in re.findall(r'\d+', ver)[:3]] + [0, 0, 0])[:3]
    return max(1, p[0] * 10000 + p[1] * 100 + p[2])


def cargo_build(target):
    """调用 cargo 交叉编译出 .so，返回路径。"""
    exe = shutil.which('cargo')
    if not exe:
        home = os.path.join(os.path.expanduser('~'), '.cargo', 'bin', 'cargo.exe')
        if os.path.exists(home):
            exe = home
        else:
            print('[失败] 找不到 cargo，请先安装 Rust 工具链')
            sys.exit(1)
    env = dict(os.environ)
    env['PATH'] = os.path.dirname(exe) + os.pathsep + env.get('PATH', '')
    # 注意：不要用 RUSTFLAGS 环境变量加链接参数——它会覆盖 .cargo/config.toml
    # 里的 link-arg=--target=...，clang 拿不到 target 就按主机模式链接，直接报
    # "unknown argument"。链接期加固参数走 `cargo rustc --` 附加，只影响最终
    # 链接的 .so，依赖照常用 config 编译。
    # Cargo.toml 里 crate-type 只写 rlib（Windows 上同时产 cdylib 会撞 GNU ld 的
    # 导出序号上限）。.so 必须用 cargo rustc 显式指定 crate-type 单独产出。
    # 安全加固（对应安全检测报告「源文件安全」项）：
    #   -z relro -z now —— Full RELRO：GOT 表只读、启动期解析完所有符号，
    #   防运行时篡改导入表劫持函数调用
    cmd = [exe, 'rustc', '--release', '--target', target, '--lib',
           '--crate-type', 'cdylib', '--features', 'native',
           '--', '-C', 'link-arg=-Wl,-z,relro', '-C', 'link-arg=-Wl,-z,now']
    print('\n>> cargo %s' % ' '.join(cmd[1:]))
    print('   （首次编译依赖较多，可能需要几分钟）')
    p = subprocess.run(cmd, cwd=RUST_DIR, env=env)
    if p.returncode != 0:
        print('\n[失败] cargo 编译失败')
        sys.exit(1)
    so = os.path.join(TARGET_DIR, target, 'release', 'lib%s.so' % LIB_NAME)
    if not os.path.exists(so):
        print('[失败] 没有产出 %s' % so)
        sys.exit(1)
    return so


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--version', default='2.0.0')
    ap.add_argument('--code', type=int, default=None,
                    help='覆盖 versionCode。Android 只认这个数判断升降级，'
                         '跨技术栈替换时必须显式抬高，versionName 不参与比较')
    ap.add_argument('--abi', default='arm64', choices=['arm64', 'armv7', 'both'])
    ap.add_argument('--no-build', action='store_true', help='跳过 cargo，复用已有 .so')
    a = ap.parse_args()

    ver = a.version
    vcode = a.code if a.code else version_code(ver)
    targets = ['arm64'] if a.abi == 'arm64' else (['armv7'] if a.abi == 'armv7' else ['arm64', 'armv7'])
    t0 = time.time()

    # ---- 1. 编译 Rust ----
    libs = {}
    for key in targets:
        tri, abi = ABIS[key]
        if a.no_build:
            so = os.path.join(TARGET_DIR, tri, 'release', 'lib%s.so' % LIB_NAME)
            if not os.path.exists(so):
                print('[失败] --no-build 但找不到 %s' % so)
                sys.exit(1)
            print('\n>> 复用 %s' % so)
        else:
            so = cargo_build(tri)
        libs[abi] = so
        print('   %-14s %8.2f MB' % (abi, os.path.getsize(so) / 1048576.0))

    # ---- 2. ASCII 工作区 ----
    if os.path.exists(STAGE):
        shutil.rmtree(STAGE)
    os.makedirs(STAGE)
    res_stage = os.path.join(STAGE, 'res')
    shutil.copytree(RES_SRC, res_stage)
    mf = os.path.join(STAGE, 'AndroidManifest.xml')
    with open(os.path.join(RUST_DIR, 'app', 'AndroidManifest.xml'), encoding='utf-8') as f:
        mx = f.read()
    mx = re.sub(r'android:versionCode="[^"]*"', 'android:versionCode="%d"' % vcode, mx)
    mx = re.sub(r'android:versionName="[^"]*"', 'android:versionName="%s"' % ver, mx)
    with open(mf, 'w', encoding='utf-8') as f:
        f.write(mx)

    KS = os.path.join(STAGE, 'release.jks')
    if not os.path.exists(KS_SRC):
        print('[失败] 缺少签名密钥 %s' % KS_SRC)
        print('       必须用原密钥签名，否则无法覆盖升级旧版')
        sys.exit(2)
    shutil.copy2(KS_SRC, KS)

    # ---- 3. aapt2 ----
    res_zip = os.path.join(STAGE, 'res.zip')
    run([AAPT2, 'compile', '--dir', res_stage, '-o', res_zip], '编译资源')
    base_apk = os.path.join(STAGE, 'base.apk')
    run([AAPT2, 'link', '-o', base_apk,
         '-I', ANDROID_JAR,
         '--manifest', mf,
         '-R', res_zip,
         '--min-sdk-version', '23',
         '--target-sdk-version', '34',
         '--version-code', str(vcode),
         '--version-name', ver,
         '--auto-add-overlay'], '链接资源与清单')

    # ---- 4. 塞入 .so ----
    unaligned = os.path.join(STAGE, 'unaligned.apk')
    with zipfile.ZipFile(base_apk, 'r') as zin:
        with zipfile.ZipFile(unaligned, 'w') as zout:
            for item in zin.infolist():
                zout.writestr(item, zin.read(item.filename))
            for abi, so in libs.items():
                # 压缩存放：清单里 extractNativeLibs=true，系统会自动解压
                zout.write(so, 'lib/%s/lib%s.so' % (abi, LIB_NAME),
                           compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    print('\n>> 已写入 %d 个 ABI 的原生库' % len(libs))

    # ---- 5. 对齐 + 签名 ----
    aligned = os.path.join(STAGE, 'aligned.apk')
    run([ZIPALIGN, '-f', '-p', '4', unaligned, aligned], '字节对齐')
    signed = os.path.join(STAGE, 'signed.apk')
    run([JAVA, '-jar', SIGNER_JAR, 'sign',
         '--ks', KS, '--ks-pass', 'pass:' + KS_PASS, '--key-pass', 'pass:' + KS_PASS,
         '--ks-key-alias', KS_ALIAS,
         '--v1-signing-enabled', 'true', '--v2-signing-enabled', 'true',
         '--v3-signing-enabled', 'true',
         '--out', signed, aligned], 'APK 签名')
    run([JAVA, '-jar', SIGNER_JAR, 'verify', '--verbose', signed], '校验签名')

    # ---- 6. 输出 ----
    os.makedirs(DIST, exist_ok=True)
    out = os.path.join(DIST, '%s_v%s_rust.apk' % (APP_NAME, ver))
    if os.path.exists(out):
        os.remove(out)
    shutil.copy2(signed, out)
    size = os.path.getsize(out) / 1048576.0
    print('\n' + '=' * 54)
    print('  Rust 版构建完成  %.2f MB  %.1fs' % (size, time.time() - t0))
    print('  输出: %s' % out)
    print('  ABI: %s   versionCode: %d' % (', '.join(libs), vcode))
    print('=' * 54)


if __name__ == '__main__':
    main()
