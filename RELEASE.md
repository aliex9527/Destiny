# 发布流程

构建在 GitHub Actions 上完成，仓库只存源码，不存任何 APK。推送一个 `v*` 形式的 tag，CI 会自动完成「构建 → 签名 → 发布 Release → 上传安装包」。

## 一次性配置

需要在该仓库配置两个 Secret：**Settings → Secrets and variables → Actions → New repository secret**

| Secret 名称 | 值 |
| --- | --- |
| `KEYSTORE_BASE64` | `keystore/dlt-release.jks.b64` 文件的**全部内容**（单行，约 3.6 KB） |
| `KEYSTORE_PASSWORD` | 密钥库口令，即 `tools/build_apk.py` 中 `KS_PASS` 的值 |

`KEYSTORE_BASE64` 的生成方式（仓库里的 `.b64` 文件已经生成好，直接复制即可；若密钥更换过则重新生成）：

```bash
base64 -w0 keystore/dlt-release.jks > keystore/dlt-release.jks.b64
```

Windows 无 `base64` 时可用 PowerShell：

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("keystore/dlt-release.jks")) | Set-Content -NoNewline keystore/dlt-release.jks.b64
```

> `keystore/` 整个目录在 `.gitignore` 中，密钥与 `.b64` 都不会进仓库。Secret 一旦设置便无法回读，只能覆盖。

## 日常发布

```bash
# 1. 改版本号：tools/build_apk.py 顶部的 VERSION / VERSION_CODE（仅本地构建需要；
#    CI 会从 tag 名覆盖，versionCode 自动按 MAJOR*10000 + MINOR*100 + PATCH 计算）
python tools/build_apk.py

# 2. 提交改动
git add -A && git commit -m "v1.4.1：说明"

# 3. 打 tag 并推送，触发 CI
git tag -a v1.4.1 -m "v1.4.1"
git push origin main v1.4.1
```

约 1~2 分钟后，安装包出现在 Releases 页。

也可以不推 tag，在 **Actions → Build & Release → Run workflow** 里填入已存在的 tag 手动触发。

## 版本号规则

| 项 | 规则 |
| --- | --- |
| `versionName` | 取自 tag 去掉前缀 `v`，如 `v1.4.1` → `1.4.1` |
| `versionCode` | `MAJOR*10000 + MINOR*100 + PATCH`，`1.4.1` → `10401` |

这条规则保证 versionCode 单调递增，可覆盖安装旧版。历史包的 versionCode 是 1~9 的手工递增值，均小于该区间起点，升级不受影响。

## 本地构建

CI 与本地使用同一份 `tools/build_apk.py`，工具路径由环境变量注入，默认值即原本地工具链布局：

| 变量 | 默认值 | 用途 |
| --- | --- | --- |
| `DLT_JDK` | `toolchain/zulu17.../` | JDK 根目录 |
| `DLT_BT` | `toolchain/android-sdk/build-tools/34.0.0` | build-tools 目录 |
| `DLT_ANDROID_JAR` | `toolchain/android.jar` | 平台 jar |
| `DLT_KS` | `keystore/dlt-release.jks` | 签名密钥 |
| `DLT_KS_PASS` / `DLT_KS_ALIAS` | `micai2026` / `dlt` | 密钥口令与别名 |
| `DLT_VERSION` | `1.4.1` | 覆盖 versionName |
| `DLT_VERSION_CODE` | `9`（或由 `DLT_VERSION` 推算） | 覆盖 versionCode |
| `DLT_REQUIRE_KS` | 未设置 | 设为 `1` 时密钥缺失直接失败，CI 用以避免生成新签名 |

## 排错

| 现象 | 原因 |
| --- | --- |
| `缺少 Secrets.KEYSTORE_BASE64` | Secret 未配置，或名字拼错 |
| `签名密钥缺失` 且退出码 2 | `DLT_KS` 指向的密钥没还原成功，检查上一步日志 |
| `apksigner verify` 失败 | `KEYSTORE_PASSWORD` 与密钥库不匹配 |
| Actions 不触发 | 仓库禁用了 Actions：Settings → Actions → General → 允许所有 action |
| Release 已建但无附件 | 检查 `dist/*.apk` 是否生成，看构建步骤日志 |

> 换成新密钥会让新旧包签名不一致，已安装的旧版本无法直接覆盖升级，必须卸载重装。因此密钥务必妥善保存，不要随意重新生成。
