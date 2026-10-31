//! Destiny · Rust 原生实现
//!
//! 入口是 Android 的 `android_main`，由 `android.app.NativeActivity` 直接加载
//! `libdestiny.so`。整个 APK 不含任何 Java 源码与 dex。

pub mod app;
pub mod data;
pub mod engine;
pub mod theme;
pub mod ticket;

use eframe::egui;

/// 内嵌中文字体子集（由 `tools/make_font.py` 生成）。
/// egui 不自带中文字形，不嵌入会全部显示成方块。
const FONT_CN: &[u8] = include_bytes!("../assets/font_subset.ttf");

pub fn install_cn_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cn".to_owned(), egui::FontData::from_static(FONT_CN).into());
    if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        list.insert(0, "cn".to_owned());
    }
    if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        list.push("cn".to_owned());
    }
    ctx.set_fonts(fonts);
}

/// 移动端字号与间距：egui 默认按桌面像素设计，手机上偏大且拥挤
pub fn install_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(egui::TextStyle::Body, theme::font(14.0));
        style.text_styles.insert(egui::TextStyle::Button, theme::font(14.0));
        style.text_styles.insert(egui::TextStyle::Small, theme::font(11.5));
        style.text_styles.insert(egui::TextStyle::Heading, theme::font(18.0));
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.scroll.bar_width = 3.0;
        style.visuals.override_text_color = Some(theme::INK);
        style.visuals.panel_fill = theme::BG;
        style.visuals.window_fill = theme::BG;
    });
}

#[cfg(target_os = "android")]
fn native_options(app: Option<android_activity::AndroidApp>) -> eframe::NativeOptions {
    eframe::NativeOptions {
        android_app: app,
        ..Default::default()
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("Destiny"),
    );

    // 应用私有目录，不需要任何存储权限
    let store = android_app
        .internal_data_path()
        .map(|p| data::store_path(&p));
    log::info!("数据目录: {:?}", store);

    let _ = eframe::run_native(
        "Destiny",
        native_options(Some(android_app)),
        Box::new(move |cc| {
            install_cn_font(&cc.egui_ctx);
            install_style(&cc.egui_ctx);
            Ok(Box::new(app::App::new(store.clone())))
        }),
    );
}
