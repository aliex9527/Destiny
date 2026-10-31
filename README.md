# Destiny

一个纯本地运行的彩票数据分析工具（Android）。所有数据与计算都在设备上完成，除同步开奖公告外不向任何服务器上传内容。

- **包名** `com.micai.dlt`
- **当前版本** v1.5.0（versionCode 10500）
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

产物写入 `dist/`。脚本要求本地存在 `toolchain/`（JDK 17 + Android build-tools 34 + android.jar）与签名密钥 `keystore/dlt-release.jks`；这两样都只在本机，不随仓库分发。

> 发布：本地构建签名 → 推安装包到 `release-assets` 分支 → 推 `v*` tag，CI 取回该包、校验后发布 Release。一条命令：`python tools/release.py --version 1.5.1`。详见 [RELEASE.md](RELEASE.md)。

> aapt2 在含中文的路径下会报 `failed to open directory`，脚本因此把全部中间产物放在 ASCII 临时目录，最后再拷回 `dist/`。

## Rust 原生版（v2.1，主线）

APK 用 Rust 写：没有 WebView、没有一行 Java、APK 里没有 dex，由系统的
`android.app.NativeActivity` 直接加载 `libdestiny.so`。界面用 egui 逐像素重绘，
配色、字号、间距、圆角全部取自 `app/src/main/assets/app.css`，与原版共用一套视觉规格
（顶栏 logo、同步条、四个页签、底部图标栏的结构和文案都一致）。

```bash
python tools/build_rust_apk.py --version 2.0.0
```

产物 `dist/Destiny_v2.0.0_rust.apk`（约 3 MB，arm64）。与 Java 版**同包名、同密钥**。

> 已经装过 Rust 版（versionCode `20000`）的设备，回装 Java 版时打包必须把 versionCode 提到
> `20000` 以上，否则系统按降级拒绝安装：
> `DLT_VERSION=1.6.0 DLT_VERSION_CODE=20600 python tools/build_apk.py`。或者先卸载再装。

```
rust/
├── Cargo.toml
├── .cargo/config.toml          # 交叉编译链接器 + 国内镜像
├── app/AndroidManifest.xml     # NativeActivity + hasCode=false
├── assets/
│   ├── font_subset.ttf         中文字体子集（tools/make_font.py 生成）
│   └── draws.txt               开奖数据（tools/gen_rust_data.py 生成）
└── src/
    ├── lib.rs                  android_main 入口、字体与样式安装
    ├── app.rs                  四页签界面
    ├── theme.rs                配色、号码球、卡片
    ├── engine.rs               统计与选号引擎（逐行对应 engine.js）
    ├── data.rs                 数据模型与本地存储
    ├── ticket.rs               奖级判定
    └── main.rs                 桌面预览入口（仅开发用）
```

构建链有三个非显然的坑，都已在配置里处理：

| 坑 | 现象 | 处理 |
| --- | --- | --- |
| 路径含中文 | GNU ld 报 `cannot find ...rcgu.o` | `.cargo/config.toml` 把 `target-dir` 指到 `C:/rsbuild/destiny` |
| 缺 `dlltool` | 宿主编译 proc-macro 时报 `program not found` | `toolchain/bin-shim/dlltool.exe` 是 NDK 里 `llvm-dlltool` 的副本 |
| egui 无中文字形 | 中文全显示成方块 | `tools/make_font.py` 把系统字体裁成子集后 `include_bytes!` 进 .so |

换字体（例如方正仿宋_GB2312）：把字体文件路径传给子集脚本即可，不用改任何代码。

```bash
python tools/make_font.py --src "D:/fonts/FZFS_GB2312.ttf"
```

> 当前用的是 Windows 自带仿宋 `simfang.ttf`（本机未安装方正仿宋_GB2312）。

## 图标

当前图标为深红底 `#A80E0A`、七根圆头柱（前五根白、后两根亮蓝 `#5AA0FF`），由 `tools/icon_v13.py` 的 D3 配色渲染：

```bash
python -c "import sys; sys.path.insert(0,'tools'); import icon_v13 as K; K.render(1024,'D3').save('icon_1024.png')"
```

`tools/` 里保留了历次设计迭代的脚本（`icon_v6` ~ `icon_v13`），可回溯每一版的几何参数与配色。

## 数据说明

开奖数据来自官方公开结果，仅供个人研究与娱乐使用。本项目不提供任何购彩服务，也不对任何形式的投注行为负责。
