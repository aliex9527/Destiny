//! 桌面预览入口（仅用于开发时截图检查布局，不参与 APK 打包）
//!
//! 窗口按手机竖屏比例 390x844 开，和真机观感接近。

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([390.0, 844.0])
            .with_title("Destiny 预览"),
        ..Default::default()
    };
    eframe::run_native(
        "Destiny",
        opts,
        Box::new(|cc| {
            destiny::install_cn_font(&cc.egui_ctx);
            destiny::install_style(&cc.egui_ctx);
            Ok(Box::new(destiny::app::App::new(None)))
        }),
    )
}
