#!/usr/bin/env bash
# CI 构建入口：探测 JDK / Android SDK 后调用 tools/build_apk.py。
# 设计原则：不依赖任何 marketplace action，全部用 runner 自带环境 + 显式回退，
# 每一步都打印探测结果，失败时日志能直接看出卡在哪。
set -uo pipefail

log() { echo "[ci] $*"; }

# ---------- 1. JDK ----------
pick_jdk() {
  if [ -n "${JAVA_HOME:-}" ] && [ -x "$JAVA_HOME/bin/javac" ]; then
    echo "$JAVA_HOME"; return 0
  fi
  local c
  # 优先 17，其次任意 17 以上
  for c in /usr/lib/jvm/*17*/ /opt/hostedtoolcache/Java_Temurin-Hotspot_jdk/17*/x64 \
           /usr/lib/jvm/*21*/ /opt/hostedtoolcache/Java_Temurin-Hotspot_jdk/21*/x64 \
           /usr/lib/jvm/*/; do
    if [ -x "${c%/}/bin/javac" ]; then echo "${c%/}"; return 0; fi
  done
  return 1
}

log "== JDK 探测 =="
log "JAVA_HOME(env)=${JAVA_HOME:-<unset>}"
if JDK_DIR="$(pick_jdk)"; then
  export JAVA_HOME="$JDK_DIR"
else
  log "未找到可用 JDK，尝试 apt 安装 openjdk-17-jdk-headless"
  sudo apt-get update -qq && sudo apt-get install -y -qq openjdk-17-jdk-headless
  JDK_DIR="$(pick_jdk)" || { log "JDK 安装后仍不可用"; exit 10; }
  export JAVA_HOME="$JDK_DIR"
fi
log "JAVA_HOME=$JAVA_HOME"
"$JAVA_HOME/bin/javac" -version

# ---------- 2. Android SDK ----------
log "== Android SDK 探测 =="
log "ANDROID_HOME(env)=${ANDROID_HOME:-<unset>}  ANDROID_SDK_ROOT=${ANDROID_SDK_ROOT:-<unset>}"
if [ -z "${ANDROID_HOME:-}" ] || [ ! -d "${ANDROID_HOME:-/nonexistent}" ]; then
  for c in /usr/local/lib/android/sdk "$HOME/Android/Sdk" /opt/android-sdk; do
    if [ -d "$c" ]; then export ANDROID_HOME="$c"; break; fi
  done
fi
if [ -z "${ANDROID_HOME:-}" ] || [ ! -d "$ANDROID_HOME" ]; then
  log "runner 未预装 SDK，尝试用 sdkmanager 安装"
  SDKMANAGER="$(command -v sdkmanager || ls /usr/local/lib/android/sdk/cmdline-tools/*/bin/sdkmanager 2>/dev/null | head -1)"
  if [ -z "$SDKMANAGER" ]; then log "找不到 sdkmanager"; exit 11; fi
  export ANDROID_HOME="${ANDROID_HOME:-/usr/local/lib/android/sdk}"
  yes | "$SDKMANAGER" --licenses > /dev/null 2>&1 || true
  "$SDKMANAGER" "build-tools;34.0.0" "platforms;android-34" || exit 12
fi
log "ANDROID_HOME=$ANDROID_HOME"
ls "$ANDROID_HOME" || exit 13

# build-tools：优先 34.0.0，否则取最高版本
BT_DIR="$ANDROID_HOME/build-tools/34.0.0"
if [ ! -d "$BT_DIR" ]; then
  BT_DIR="$(ls -d "$ANDROID_HOME"/build-tools/*/ 2>/dev/null | sort -V | tail -1)"
  BT_DIR="${BT_DIR%/}"
fi
[ -d "$BT_DIR" ] || { log "找不到 build-tools"; exit 14; }
log "build-tools: $BT_DIR"
ls "$BT_DIR" | head -20

# android.jar：优先 platform 34，否则取最高已装平台
JAR="$ANDROID_HOME/platforms/android-34/android.jar"
if [ ! -f "$JAR" ]; then
  JAR="$(ls "$ANDROID_HOME"/platforms/android-*/android.jar 2>/dev/null | sort -V | tail -1)"
fi
[ -f "$JAR" ] || { log "找不到 android.jar"; exit 15; }
log "android.jar: $JAR"

# ---------- 3. 构建 ----------
log "== 开始构建 =="
export DLT_JDK="$JAVA_HOME"
export DLT_BT="$BT_DIR"
export DLT_ANDROID_JAR="$JAR"
export DLT_KS="${DLT_KS:-$PWD/keystore/dlt-release.jks}"
export DLT_REQUIRE_KS="${DLT_REQUIRE_KS:-1}"
log "DLT_VERSION=${DLT_VERSION:-<未设置>}"
log "DLT_KS=$DLT_KS  存在=$([ -f "$DLT_KS" ] && echo yes || echo no)"

python3 tools/build_apk.py || exit 20

log "== 产物 =="
ls -l dist/*.apk
sha256sum dist/*.apk
