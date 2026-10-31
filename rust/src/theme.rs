//! 视觉规格：配色、字号、号码球与卡片
//!
//! 颜色取自 web 版 `app.css` 的 CSS 变量，保持新旧两版观感连续。

use eframe::egui;
use egui::{Color32, FontId, Pos2, Response, Sense, Stroke, Ui, Vec2};

/* ---------------- 配色 ---------------- */
pub const BG: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF7);
pub const CARD: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const INK: Color32 = Color32::from_rgb(0x0F, 0x13, 0x19);
pub const INK2: Color32 = Color32::from_rgb(0x3A, 0x41, 0x50);
pub const SUB: Color32 = Color32::from_rgb(0x7A, 0x82, 0x91);
pub const LINE: Color32 = Color32::from_rgb(0xE9, 0xEB, 0xF0);
pub const RED: Color32 = Color32::from_rgb(0xDE, 0x33, 0x2B);
pub const RED_SOFT: Color32 = Color32::from_rgb(0xFD, 0xED, 0xEB);
pub const BLUE: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const BLUE_SOFT: Color32 = Color32::from_rgb(0xEA, 0xF0, 0xFE);
pub const GOLD: Color32 = Color32::from_rgb(0xE9, 0x92, 0x0B);
pub const GREEN: Color32 = Color32::from_rgb(0x12, 0xA1, 0x50);

/// 前区球底色与高光
const BALL_F: Color32 = Color32::from_rgb(0xDE, 0x33, 0x2B);
const BALL_F_HI: Color32 = Color32::from_rgb(0xFF, 0x8C, 0x7B);
/// 后区球底色与高光
const BALL_B: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
const BALL_B_HI: Color32 = Color32::from_rgb(0x7F, 0xA8, 0xFF);

pub const RADIUS: f32 = 12.0;

/* ---------------- 字体 ---------------- */

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn font_bold(size: f32) -> FontId {
    // 只嵌入了一个字重，粗体通过颜色和字号强化，避免 egui 自行合成导致糊边
    FontId::proportional(size)
}

/* ---------------- 号码球 ---------------- */

/// 画一个号码球。`front` 决定红/蓝，`dim` 用于核对页未命中的灰球。
pub fn draw_ball(painter: &egui::Painter, center: Pos2, radius: f32, n: u8, front: bool, dim: bool) {
    let (base, hi) = if dim {
        (Color32::from_rgb(0xE6, 0xE8, 0xED), Color32::from_rgb(0xF2, 0xF3, 0xF6))
    } else if front {
        (BALL_F, BALL_F_HI)
    } else {
        (BALL_B, BALL_B_HI)
    };
    painter.circle_filled(center, radius, base);
    // 左上高光模拟径向渐变
    let hi_pos = center + Vec2::new(-radius * 0.30, -radius * 0.32);
    painter.circle_filled(hi_pos, radius * 0.52, hi.gamma_multiply(if dim { 0.5 } else { 0.45 }));

    let txt_color = if dim { Color32::from_rgb(0x9A, 0xA1, 0xAC) } else { Color32::WHITE };
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        format!("{:02}", n),
        font(radius * 0.92),
        txt_color,
    );
}

/// 一行号码球：前区 5 个 + 分隔线 + 后区 2 个
///
/// 间距按 web 版规格换算：球直径 33、间距 7、分隔区 5。
pub fn balls_row(ui: &mut Ui, f: &[u8], b: &[u8], radius: f32, dim_mask: Option<&[bool]>) -> Response {
    let d = radius * 2.0;
    let gap = radius * 0.42;
    let sep = radius * 0.30;
    let total = f.len() + b.len();
    let width = d * total as f32 + gap * (total as f32 - 1.0) + sep;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, d), Sense::hover());
    let painter = ui.painter();
    let cy = rect.center().y;

    let dim_at = |i: usize| -> bool {
        dim_mask.map(|m| !m.get(i).copied().unwrap_or(true)).unwrap_or(false)
    };

    let mut x = rect.left() + radius;
    for (i, &n) in f.iter().enumerate() {
        draw_ball(painter, Pos2::new(x, cy), radius, n, true, dim_at(i));
        x += d + gap;
    }

    // 前区排完，x 已越过一个 gap；把分隔线放在剩余空隙中央
    let sep_x = x - gap * 0.5;
    painter.line_segment(
        [Pos2::new(sep_x, cy - radius * 0.62), Pos2::new(sep_x, cy + radius * 0.62)],
        Stroke::new(1.0, LINE),
    );
    x += sep - gap * 0.5;

    for (i, &n) in b.iter().enumerate() {
        draw_ball(painter, Pos2::new(x, cy), radius, n, false, dim_at(f.len() + i));
        x += d + gap;
    }
    resp
}

/* ---------------- 卡片与按钮 ---------------- */

/// 白底圆角卡片，返回内容区可用矩形
pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::symmetric(14, 13))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// 主按钮（红底白字）
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let h = 46.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0xC2, 0x28, 0x21)
    } else {
        RED
    };
    ui.painter().rect_filled(rect, 13.0, bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font(15.5),
        Color32::WHITE,
    );
    resp
}

/// 次要按钮（白底描边）
pub fn ghost_button(ui: &mut Ui, text: &str, width: Option<f32>) -> Response {
    let h = 34.0;
    let w = width.unwrap_or(ui.available_width());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let fill = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0xF7, 0xF8, 0xFA)
    } else {
        CARD
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    ui.painter().rect_stroke(rect, 10.0, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font(13.0),
        INK2,
    );
    resp
}

/// 小标签（策略 chip、注数标签等）
pub fn chip(ui: &mut Ui, text: &str, active: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.5), INK2);
    let w = galley.size().x + 22.0;
    let h = 30.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let (fill, stroke_c, txt) = if active {
        (RED_SOFT, RED, RED)
    } else {
        (CARD, LINE, INK2)
    };
    ui.painter().rect_filled(rect, 9.0, fill);
    ui.painter().rect_stroke(rect, 9.0, Stroke::new(1.0, stroke_c), egui::StrokeKind::Inside);
    ui.painter()
        .text(rect.center(), egui::Align2::CENTER_CENTER, text, font(12.5), txt);
    resp
}

/// 分隔线
pub fn hairline(ui: &mut Ui) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
    ui.painter()
        .line_segment([rect.left_center(), rect.right_center()], Stroke::new(1.0, LINE));
}

/// 一段说明文字，自动换行
pub fn note(ui: &mut Ui, text: &str, color: Color32, size: f32) {
    ui.add(
        egui::Label::new(egui::RichText::new(text).color(color).size(size))
            .wrap()
            .selectable(false),
    );
}
