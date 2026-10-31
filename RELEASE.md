# 发布流程

构建在 GitHub Actions 上完成，仓库只存源码，不存任何 APK。推送一个 `v*` 形式的 tag，CI 会自动完成「构建 → 签名 → 发布 Release → 上传安装包」。

## 密钥放置

**不需要任何 Secret。** 签名密钥 `keystore/dlt-release.jks` 直接提交在仓库里，CI checkout 后即可签名。

为什么这样：Android 包必须用同一把密钥签名才能覆盖升级。此前走 Secrets（`KEYSTORE_BASE64`）方案，但仓库重建时 GitHub 会清空 Secrets，且 fine-grained token 不支持 Secrets API，无法脚本化恢复，反复卡在这一步。本仓库为私有，故改为密钥入库，换取发布链路零配置。

代价：**任何能读这个仓库的人都能签出可覆盖安装的更新包**。所以

- 不要把仓库转为公开，也慎加协作者
- 若将来必须开源，先把密钥移回 Secrets 并更换新密钥（老用户需卸载重装一次）

口令与别名仍沿用 `tools/build_apk.py` 内的默认值 `micai2026` / `dlt`，可用 `DLT_KS_PASS` / `DLT_KS_ALIAS` 覆盖。

密钥完整性由 CI 第一步打印 sha256，历史值见对应 tag 的构建日志。

```bash
# CI 里实际执行的校验（tools/ci_build.sh 内部）
sha256sum keystore/dlt-release.jks
```

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
| `仓库中缺少 keystore/dlt-release.jks` | 密钥没提交进仓库，或 checkout 被裁剪 |
| `签名密钥缺失` 且退出码 2 | `DLT_KS` 指向的路径不存在，看构建日志第一步的探测结果 |
| `apksigner verify` 失败 | 密钥库口令不对（`KS_PASS` / `DLT_KS_PASS`） |
| `build outcome: failure` 但日志看不到 | 拉到 `ci-logs` 分支：`git fetch origin ci-logs` 后看 `.ci/ci-build.log` |
| Actions 不触发 | 仓库禁用了 Actions：Settings → Actions → General → 允许所有 action |
| Release 已建但无附件 | 检查 `dist/*.apk` 是否生成，看构建步骤日志 |
| 无密钥时想单独验证构建 | 推 `smoke-*` tag，走 `smoke.yml`，自动生成临时签名且不发布 |

> 换成新密钥会让新旧包签名不一致，已安装的旧版本无法直接覆盖升级，必须卸载重装。因此密钥务必妥善保存，不要随意重新生成。
