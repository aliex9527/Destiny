#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
运 APK 构建脚本（无 Gradle 依赖）
aapt2 -> javac -> d8 -> 打包 dex -> zipalign -> apksigner

注意：aapt2 在包含中文的路径下会报 "failed to open directory"，
因此所有中间产物都在 ASCII 临时目录完成，最后再把 APK 拷回 dist/。

用法：python tools/build_apk.py
"""
import os, re, sys, shutil, subprocess, zipfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOL = os.path.join(ROOT, 'toolchain')
JDK = os.path.join(TOOL, 'zulu17.54.21-ca-jdk17.0.13-win_x64')
BT = os.path.join(TOOL, 'android-sdk', 'build-tools', '34.0.0')
ANDROID_JAR = os.path.join(TOOL, 'android.jar')
APP = os.path.join(ROOT, 'app')
DIST = os.path.join(ROOT, 'dist')
KS_SRC = os.path.join(ROOT, 'keystore', 'dlt-release.jks')

VERSION = '1.4.0'
VERSION_CODE = '8'
APP_NAME = '运'
KS_PASS = 'micai2026'
KS_ALIAS = 'dlt'

STAGE = os.path.join(os.environ.get('TEMP', 'C:\\temp'), 'dltbuild')
BUILD = os.path.join(STAGE, 'build')
KS = os.path.join(STAGE, 'dlt-release.jks')

JAVA = os.path.join(JDK, 'bin', 'java.exe')
JAVAC = os.path.join(JDK, 'bin', 'javac.exe')
KEYTOOL = os.path.join(JDK, 'bin', 'keytool.exe')
AAPT2 = os.path.join(BT, 'aapt2.exe')
ZIPALIGN = os.path.join(BT, 'zipalign.exe')
D8_JAR = os.path.join(BT, 'lib', 'd8.jar')
SIGNER_JAR = os.path.join(BT, 'lib', 'apksigner.jar')
AAPT = os.path.join(BT, 'aapt.exe')


def run(cmd, desc, cwd=None):
    print('\n>> %s' % desc)
    p = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    out = p.stdout.decode('utf-8', 'ignore').strip()
    if out:
        print(out[-2500:])
    if p.returncode != 0:
        print('\n[失败] %s  (exit %d)' % (desc, p.returncode))
        sys.exit(1)
    return out


def main():
    t0 = time.time()

    # ---- 准备 ASCII 工作区 ----
    if os.path.exists(STAGE):
        shutil.rmtree(STAGE)
    os.makedirs(STAGE)
    if os.path.exists(KS_SRC):
        shutil.copy2(KS_SRC, KS)
        print('>> 复用已有签名密钥')
    app_stage = os.path.join(STAGE, 'app')
    shutil.copytree(APP, app_stage)

    # 清单里的 versionCode/versionName 才是 aapt2 实际采用的，统一由上面的常量写入
    mf = os.path.join(app_stage, 'AndroidManifest.xml')
    with open(mf, encoding='utf-8') as f:
        mx = f.read()
    mx = re.sub(r'android:versionCode="[^"]*"', 'android:versionCode="%s"' % VERSION_CODE, mx)
    mx = re.sub(r'android:versionName="[^"]*"', 'android:versionName="%s"' % VERSION, mx)
    with open(mf, 'w', encoding='utf-8') as f:
        f.write(mx)
    for sub in ('build/res', 'build/gen', 'build/classes', 'build/dex'):
        os.makedirs(os.path.join(STAGE, sub))

    # 0. keystore
    if not os.path.exists(KS):
        run([KEYTOOL, '-genkeypair', '-keystore', KS, '-alias', KS_ALIAS,
             '-keyalg', 'RSA', '-keysize', '2048', '-validity', '10000',
             '-storepass', KS_PASS, '-keypass', KS_PASS,
             '-dname', 'CN=Micai, OU=Mobile, O=Micai Studio, L=Beijing, ST=Beijing, C=CN'],
            '生成签名密钥')

    # 1. aapt2 compile
    res_zip = os.path.join(BUILD, 'res.zip')
    run([AAPT2, 'compile', '--dir', os.path.join(app_stage, 'res'), '-o', res_zip], '编译资源')

    # 2. aapt2 link
    base_apk = os.path.join(BUILD, 'base.apk')
    run([AAPT2, 'link', '-o', base_apk,
         '-I', ANDROID_JAR,
         '--manifest', os.path.join(app_stage, 'AndroidManifest.xml'),
         '-R', res_zip,
         '-A', os.path.join(app_stage, 'src', 'main', 'assets'),
         '--java', os.path.join(BUILD, 'gen'),
         '--min-sdk-version', '23',
         '--target-sdk-version', '34',
         '--version-code', VERSION_CODE,
         '--version-name', VERSION,
         '--auto-add-overlay'], '链接资源与清单')

    # 3. javac
    java_files = []
    for base in (os.path.join(BUILD, 'gen'), os.path.join(app_stage, 'src')):
        for dp, _, fns in os.walk(base):
            for fn in fns:
                if fn.endswith('.java'):
                    java_files.append(os.path.join(dp, fn))
    print('\n>> 编译 Java 源码 (%d 个文件)' % len(java_files))
    run([JAVAC, '-encoding', 'UTF-8', '-source', '8', '-target', '8',
         '-bootclasspath', ANDROID_JAR, '-classpath', ANDROID_JAR,
         '-d', os.path.join(BUILD, 'classes')] + java_files, 'javac')

    # 4. d8
    classes = []
    for dp, _, fns in os.walk(os.path.join(BUILD, 'classes')):
        for fn in fns:
            if fn.endswith('.class'):
                classes.append(os.path.join(dp, fn))
    dex_dir = os.path.join(BUILD, 'dex')
    run([JAVA, '-cp', D8_JAR, 'com.android.tools.r8.D8',
         '--release', '--min-api', '23', '--lib', ANDROID_JAR,
         '--output', dex_dir] + classes, 'dex 转换 (d8)')

    # 5. 合并 dex
    unaligned = os.path.join(BUILD, 'unaligned.apk')
    dex_file = os.path.join(dex_dir, 'classes.dex')
    if not os.path.exists(dex_file):
        print('[失败] 没有生成 classes.dex')
        sys.exit(1)
    with zipfile.ZipFile(base_apk, 'r') as zin:
        with zipfile.ZipFile(unaligned, 'w') as zout:
            for item in zin.infolist():
                zout.writestr(item, zin.read(item.filename))
            zout.write(dex_file, 'classes.dex', compress_type=zipfile.ZIP_STORED)
    print('\n>> 合并 classes.dex (%.2f MB)' % (os.path.getsize(dex_file) / 1048576.0))

    # 6. zipalign
    aligned = os.path.join(BUILD, 'aligned.apk')
    run([ZIPALIGN, '-f', '-p', '4', unaligned, aligned], '字节对齐')

    # 7. 签名
    signed = os.path.join(BUILD, 'signed.apk')
    run([JAVA, '-jar', SIGNER_JAR, 'sign',
         '--ks', KS, '--ks-pass', 'pass:' + KS_PASS, '--key-pass', 'pass:' + KS_PASS,
         '--ks-key-alias', KS_ALIAS,
         '--v1-signing-enabled', 'true', '--v2-signing-enabled', 'true',
         '--v3-signing-enabled', 'true',
         '--out', signed, aligned], 'APK 签名')

    # 8. 校验
    print('\n>> 校验签名')
    run([JAVA, '-jar', SIGNER_JAR, 'verify', '--verbose', signed], 'apksigner verify')
    if os.path.exists(AAPT):
        info = run([AAPT, 'dump', 'badging', signed], 'aapt badging')
        for line in info.splitlines():
            if line.startswith(('package:', 'application-label', 'launchable-activity',
                                'sdkVersion', 'targetSdkVersion')):
                print('   ' + line)

    # 9. 回拷
    if not os.path.exists(DIST):
        os.makedirs(DIST)
    out_apk = os.path.join(DIST, '%s_v%s.apk' % (APP_NAME, VERSION))
    if os.path.exists(out_apk):
        os.remove(out_apk)
    shutil.copy2(signed, out_apk)
    # 保存 keystore 以便下次同签名升级
    if not os.path.exists(KS_SRC):
        os.makedirs(os.path.dirname(KS_SRC))
        shutil.copy2(KS, KS_SRC)

    size = os.path.getsize(out_apk) / 1048576.0
    print('\n' + '=' * 54)
    print('  构建完成  %.1f MB  %.1fs' % (size, time.time() - t0))
    print('  输出: %s' % out_apk)
    print('=' * 54)


if __name__ == '__main__':
    main()
