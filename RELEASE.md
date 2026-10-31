# 发布流程

**构建在本地完成，CI 只负责分发。**

签名密钥只保存在本机，不进仓库、不进 CI。因此发布分两段：本地构建并签名 → CI 取回安装包发布 Release。

```bash
python tools/release.py --version 1.5.1
```

一条命令覆盖：构建 → 签名 → 推送安装包到 `release-assets` 分支 → 打 tag → 推 tag 触发 CI。约 30 秒后安装包出现在 Releases 页。

## 密钥放置

| 项 | 做法 |
| --- | --- |
| 位置 | `keystore/dlt-release.jks`，仅本机 |
| 是否入库 | **否**，`keystore/` 整个目录在 `.gitignore` 内；历史提交中的副本也已清除 |
| 是否配 Secret | **否**，CI 不构建，用不到密钥 |
| 口令 / 别名 | `micai2026` / `dlt`（`tools/build_apk.py` 默认值，可用 `DLT_KS_PASS` / `DLT_KS_ALIAS` 覆盖） |

**务必自行备份这个文件。** Android 包必须用同一把密钥签名才能覆盖升级：密钥丢失就再也无法为已安装的用户发布更新，只能换新密钥、让所有人卸载重装。备份到仓库之外的地方（密码管理器、加密U盘），不要放进任何 git 仓库。

> 早先的版本曾把密钥提交进私有仓库以便 CI 直接构建，现已撤销：仓库历史中的该文件已被清除，`release-assets` 分支只存放已签名的安装包（即对外分发的产物本身，不含密钥）。

## 发布链路

```
本地                                    GitHub Actions
─────────────────────────────────────   ──────────────────────────────
tools/release.py
  ├─ build_apk.py 构建 + 本地签名
  ├─ APK → release-assets 分支（孤儿提交，强制覆盖）
  └─ 打 tag 并推送  ───────────────────▶  取回 release-assets 里的 APK
                                          核对包内 versionName == tag
                                          apksigner 校验签名
                                          创建 Release + 上传附件
                                          日志推回 ci-logs 分支
```

`release-assets` 每次都是**单提交**，强制覆盖，不累积历史；它不参与 `main`，也不影响代码目录结构。

## 日常发布

```bash
# 改代码后提交
git add -A && git commit -m "说明"
git push origin main

# 一键发布（默认版本号取 tools/build_apk.py 顶部 VERSION）
python tools/release.py --version 1.5.1
```

`tools/release.py` 的开关：

| 参数 | 作用 |
| --- | --- |
| `--version 1.5.1` | 指定版本号，不传则取 `build_apk.py` 的 `VERSION` |
| `--no-build` | 复用 `dist/` 里已构建好的包，只推包与 tag |
| `--retag` | 允许覆盖已存在的同名 tag（重发同一版本时用） |
| `--no-push` | 只构建、只打本地 tag，不联网 |

`versionName` 取版本号，`versionCode` 按 `MAJOR*10000 + MINOR*100 + PATCH` 计算（`1.5.1` → `10501`），保证单调递增、可覆盖安装旧版。历史包是 1~9 的手工递增值，均小于该区间起点，不受影响。

## 不想动 CI 时的手工流程

```bash
python tools/build_apk.py                     # 只构建，产物在 dist/
python tools/release.py --no-build --no-push  # 只打本地 tag，看清要推什么
git push -f origin v1.5.1                     # 推 tag 触发发布
```

## 排错

| 现象 | 原因 |
| --- | --- |
| `缺少签名密钥` 且退出码 2 | 本机 `keystore/dlt-release.jks` 不存在，从备份恢复 |
| `apksigner verify` 失败 | 密钥口令不对（`KS_PASS` / `DLT_KS_PASS`） |
| CI 报 `远端没有 release-assets 分支` | 没先跑 `tools/release.py` 推包，或只推了 tag |
| CI 报 `包内版本与 tag 不一致` | 推的包版本与 tag 不符，检查 `--version` 是否与构建产物对应 |
| 推了 tag 但 Actions 没动静 | 强制更新已有 tag 不一定触发；`tools/release.py` 已自动改为「先删远端 tag 再推」，手工推时照做 |
| `release outcome: failure` | 拉日志：`git fetch origin ci-logs && git show origin/ci-logs:.ci/ci-build.log` |
| Actions 不触发 | 仓库禁用了 Actions：Settings → Actions → General → 允许所有 action |
| 想验证 Linux 构建链路 | 推 `smoke-*` tag，走 `smoke.yml`，用临时密钥构建、不发布 |

## 本地构建依赖

工具路径由环境变量注入，默认值即本地工具链布局：

| 变量 | 默认值 | 用途 |
| --- | --- | --- |
| `DLT_JDK` | `toolchain/zulu17.../` | JDK 根目录 |
| `DLT_BT` | `toolchain/android-sdk/build-tools/34.0.0` | build-tools 目录 |
| `DLT_ANDROID_JAR` | `toolchain/android.jar` | 平台 jar |
| `DLT_KS` | `keystore/dlt-release.jks` | 签名密钥 |
| `DLT_KS_PASS` / `DLT_KS_ALIAS` | `micai2026` / `dlt` | 密钥口令与别名 |
| `DLT_VERSION` | `1.5.0` | 覆盖 versionName |
| `DLT_VERSION_CODE` | 由 `DLT_VERSION` 推算 | 覆盖 versionCode |
| `DLT_REQUIRE_KS` | 未设置 | 设为 `1` 时密钥缺失直接失败，避免误生成新签名 |
