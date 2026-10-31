# 运

一个纯本地运行的彩票数据分析工具（Android）。所有数据与计算都在设备上完成，除同步开奖公告外不向任何服务器上传内容。

- **包名** `com.micai.dlt`
- **当前版本** v1.4.0（versionCode 8）
- **最低系统** Android 6.0（API 23），目标 API 34
- **安装包** 见 [Releases](../../releases) 页下载；或本地执行 `python tools/build_apk.py` 自行构建（仓库不存放二进制）

## 功能

| 模块 | 说明 |
| --- | --- |
| 开奖数据 | 内置历史开奖数据，可按期号或日期检索；联网同步官方最新开奖 |
| 统计 | 前区/后区冷热、遗漏值、区间分布、和值、奇偶与大小比 |
| 选号 | 多种策略生成号码，支持单式与复式 |
| 验票 | 按当前 7 个奖级规则自动判奖，可手动点选或拍照录入 |
| 提醒 | 开奖日（周一 / 三 / 六）21:45 查询并提醒 |

## 架构

Java 只做容器，界面与业务逻辑全部在 WebView 内的 assets 中，因此改界面不需要重新编译 Java。

```
app/
├── AndroidManifest.xml
├── res/
│   ├── mipmap-*/ic_launcher.png   启动图标（mdpi 48 → xxxhdpi 192）
│   ├── drawable/ic_stat.xml       通知栏小图标
│   └── values/                    应用名、主题
├── src/com/micai/dlt/
│   ├── MainActivity.java          WebView 容器，shouldInterceptRequest 把
│   │                              https://dlt.local/* 映射到 assets，
│   │                              让 localStorage 有稳定 origin
│   ├── Net.java                   原生 HTTP 代理，绕开 WebView 跨域限制
│   ├── PhotoProvider.java         拍票时写入缓存的 FileProvider
│   ├── Remind.java                开奖日闹钟
│   └── DrawCheckReceiver.java     开机恢复闹钟
└── main/assets/
    ├── index.html                 单页骨架
    ├── app.css
    ├── app.js                     交互与渲染
    ├── engine.js                  统计与选号引擎
    ├── ticket.js                  验票引擎
    └── data.js                    内置开奖数据（脚本生成，勿手改）
```

## 构建

不需要 Gradle，构建脚本直接调用 Android SDK 工具链（`aapt2 → javac → d8 → zipalign → apksigner`）。

```bash
python tools/build_apk.py
```

产物写入 `dist/`。脚本要求本地存在 `toolchain/`（JDK 17 + Android build-tools 34 + android.jar）与 `keystore/dlt-release.jks`；这两样不随仓库分发。

> 发版走 CI：推送 `v*` tag 后由 GitHub Actions 构建并发布 Release，配置方法与版本号规则见 [RELEASE.md](RELEASE.md)。

> aapt2 在含中文的路径下会报 `failed to open directory`，脚本因此把全部中间产物放在 ASCII 临时目录，最后再拷回 `dist/`。

## 图标

当前图标为深红底 `#A80E0A`、七根圆头柱（前五根白、后两根亮蓝 `#5AA0FF`），由 `tools/icon_v13.py` 的 D3 配色渲染：

```bash
python -c "import sys; sys.path.insert(0,'tools'); import icon_v13 as K; K.render(1024,'D3').save('icon_1024.png')"
```

`tools/` 里保留了历次设计迭代的脚本（`icon_v6` ~ `icon_v13`），可回溯每一版的几何参数与配色。

## 数据说明

开奖数据来自官方公开结果，仅供个人研究与娱乐使用。本项目不提供任何购彩服务，也不对任何形式的投注行为负责。
