//! 视觉层：配色、字体、号码球、卡片、按钮、图标、图表元件
//!
//! 所有数值都直接取自 web 版 `app.css`，单位是 CSS px，与 egui 的点在
//! Android 上基本 1:1（dp）。改这里的常量等于改 `app.css`，两版观感一致。

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

/* ============================ 配色 ============================ */

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
/// 免责声明与最弱的辅助文字
pub const FAINT: Color32 = Color32::from_rgb(0x9A, 0xA1, 0xAC);
/// 底栏未选中色
pub const TAB_IDLE: Color32 = Color32::from_rgb(0x9A, 0xA1, 0xAC);
/// 分段控件底
pub const SEG_BG: Color32 = Color32::from_rgb(0xF2, 0xF3, 0xF6);
/// 浅灰块（球槽、开关底）
pub const CHIP_BG: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF7);

/* 号码球渐变端点（CSS 里的 radial-gradient 三档） */
const BALL_F_HI: Color32 = Color32::from_rgb(0xFF, 0x8C, 0x7B);
const BALL_F_LO: Color32 = Color32::from_rgb(0xAE, 0x1C, 0x16);
const BALL_B_HI: Color32 = Color32::from_rgb(0x7F, 0xA8, 0xFF);
const BALL_B_LO: Color32 = Color32::from_rgb(0x1B, 0x46, 0xB8);

/* ============================ 尺寸 ============================ */

pub const CARD_R: f32 = 18.0;
pub const PAGE_PAD: f32 = 14.0;
pub const HEADER_H: f32 = 56.0;
pub const SYNC_H: f32 = 34.0;
pub const TABBAR_H: f32 = 58.0;
/// 号码球直径（CSS .ball 33px）
pub const BALL_D: f32 = 33.0;

/* ============================ 字体 ============================ */

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

/// 粗体：只嵌了一个字重，egui 不支持合成加粗，
/// 这里靠字号与颜色拉伸权重感，避免假粗体糊边。
pub fn bold(size: f32) -> FontId {
    FontId::proportional(size)
}

/* ============================ 基础形状 ============================ */

/// 垂直/水平线性渐变矩形。EGUI 没有原生渐变，用两个三角形拼。
pub fn gradient(p: &egui::Painter, rect: Rect, from: Color32, to: Color32, vertical: bool) {
    let mut mesh = egui::Mesh::default();
    let base = mesh.vertices.len() as u32;
    let (a, b, c, d) = if vertical {
        (rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom())
    } else {
        (rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom())
    };
    let (c0, c1) = if vertical { (from, to) } else { (from, to) };
    mesh.colored_vertex(a, c0);
    mesh.colored_vertex(b, if vertical { c0 } else { c1 });
    mesh.colored_vertex(c, c1);
    mesh.colored_vertex(d, if vertical { c1 } else { c0 });
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base, base + 2, base + 3);
    p.add(Shape::mesh(mesh));
}

/// 水平条：底槽 + 填充，用于柱状图与遗漏榜
pub fn hbar(p: &egui::Painter, rect: Rect, ratio: f32, from: Color32, to: Color32) {
    p.rect_filled(rect, rect.height() * 0.5, SEG_BG);
    let w = (rect.width() * ratio.clamp(0.0, 1.0)).max(rect.height());
    if ratio > 0.001 {
        let fill = Rect::from_min_size(rect.min, Vec2::new(w, rect.height()));
        gradient(p, fill, from, to, false);
        // 圆角用同色小圆补两端，避免 mesh 直角露出
        p.circle_filled(Pos2::new(fill.left() + rect.height() * 0.5, fill.center().y), rect.height() * 0.5, from);
        p.circle_filled(Pos2::new(fill.right() - rect.height() * 0.5, fill.center().y), rect.height() * 0.5, to);
    }
}

/* ============================ 号码球 ============================ */

/// 画一个号码球。`front` 决定红/蓝，`dim` 用于未命中。
pub fn draw_ball(p: &egui::Painter, center: Pos2, d: f32, n: u8, front: bool, dim: bool) {
    let r = d * 0.5;
    let (hi, mid, lo) = if dim {
        (
            Color32::from_rgb(0xF2, 0xF3, 0xF6),
            Color32::from_rgb(0xE6, 0xE8, 0xED),
            Color32::from_rgb(0xE6, 0xE8, 0xED),
        )
    } else if front {
        (BALL_F_HI, RED, BALL_F_LO)
    } else {
        (BALL_B_HI, BLUE, BALL_B_LO)
    };

    // 径向渐变：底色 + 由大到小的偏移圆叠出左侧高光
    p.circle_filled(center, r, mid);
    p.circle_filled(center, r, lo.gamma_multiply(0.0)); // 占位，保持层级直觉
    let steps = 5;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let rr = r * (1.0 - t * 0.62);
        let c = hi.gamma_multiply(1.0 - t * 0.55).lerp_to_gamma(mid, t * 0.75);
        let ctr = center + Vec2::new(-r * 0.30 * t, -r * 0.30 * t);
        p.circle_filled(ctr, rr, c);
    }

    // 底部内阴影：CSS box-shadow inset 0 -1.5px 2px rgba(0,0,0,.16)
    if !dim {
        p.circle_filled(
            center + Vec2::new(0.0, -r * 0.42),
            r * 0.96,
            Color32::TRANSPARENT,
        );
        p.circle_filled(center, r, Color32::TRANSPARENT);
        // 用一圈细弧模拟内阴影，成本很低
        let pts: Vec<Pos2> = (0..=12)
            .map(|i| {
                let a = std::f32::consts::PI * (i as f32 / 12.0) + std::f32::consts::PI;
                center + Vec2::new(a.cos() * (r - 0.6), a.sin() * (r - 0.6))
            })
            .collect();
        for w in pts.windows(2) {
            p.line_segment([w[0], w[1]], Stroke::new(1.4, Color32::from_black_alpha(26)));
        }
    }

    let txt = if dim { FAINT } else { Color32::WHITE };
    p.text(
        center,
        Align2::CENTER_CENTER,
        format!("{:02}", n),
        bold(d * 0.44),
        txt,
    );
}

/// 一行号码球：前区 + 竖分隔线 + 后区。返回占用的矩形。
pub fn balls_row(
    ui: &mut Ui,
    f: &[u8],
    b: &[u8],
    d: f32,
    dim_mask: Option<&[bool]>,
) -> Rect {
    let gap = if d >= 30.0 { 7.0 } else { 6.0 };
    let sep_w = 7.0;
    let total = f.len() + b.len();
    let width = d * total as f32 + gap * (total.saturating_sub(1)) as f32 + sep_w * 2.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, d), Sense::hover());
    let p = ui.painter();
    let cy = rect.center().y;
    let dim_at =
        |i: usize| dim_mask.map(|m| !m.get(i).copied().unwrap_or(true)).unwrap_or(false);

    let mut x = rect.left() + d * 0.5;
    for (i, &n) in f.iter().enumerate() {
        draw_ball(p, Pos2::new(x, cy), d, n, true, dim_at(i));
        x += d + gap;
    }
    // 分隔线落在两段之间
    x -= gap;
    let sep_x = x + gap * 0.5;
    p.line_segment(
        [
            Pos2::new(sep_x, cy - 10.0),
            Pos2::new(sep_x, cy + 10.0),
        ],
        Stroke::new(1.0, LINE),
    );
    x = sep_x + gap * 0.5 + d * 0.5;
    for (i, &n) in b.iter().enumerate() {
        draw_ball(p, Pos2::new(x, cy), d, n, false, dim_at(f.len() + i));
        x += d + gap;
    }
    rect
}

/* ============================ 卡片 ============================ */

/// 白底圆角卡片，1px 边框
pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    card_tinted(ui, CARD, add);
}

pub fn card_tinted(ui: &mut Ui, fill: Color32, add: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let start = ui.cursor().min;
    let resp = egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CARD_R)
        .inner_margin(egui::Margin::symmetric(14, 14))
        .show(ui, |ui| {
            ui.set_width(w - 28.0);
            add(ui);
        });
    let _ = start;
    ui.add_space(12.0);
    let _ = resp;
}

/// 段落标题（卡片外的灰黑小标题）
pub fn sec_title(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), bold(13.0), INK2);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), galley.size().y + 8.0),
        Sense::hover(),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 2.0, rect.top()),
        Align2::LEFT_TOP,
        text,
        bold(13.0),
        INK2,
    );
}

/// 自动换行的说明文字
pub fn para(ui: &mut Ui, text: &str, size: f32, color: Color32, line: f32) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(size)
                .color(color)
                .line_height(Some(size * line)),
        )
        .wrap()
        .selectable(false),
    );
}

/// 免责声明
pub fn disclaimer(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 4.0);
            para(ui, text, 11.0, FAINT, 1.65);
        });
    });
    ui.add_space(6.0);
}

/* ============================ 按钮 ============================ */

/// 主按钮：红色渐变、15 圆角、白字 16
pub fn primary(ui: &mut Ui, text: &str, icon: Option<fn(&egui::Painter, Rect, Color32)>) -> Response {
    let w = ui.available_width();
    let h = 50.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let p = ui.painter();
    let top = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0xD8, 0x2A, 0x22)
    } else {
        Color32::from_rgb(0xF0, 0x50, 0x3F)
    };
    let bot = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0xB4, 0x1C, 0x15)
    } else {
        Color32::from_rgb(0xCB, 0x21, 0x19)
    };
    // 圆角矩形打底，渐变叠在内部，避免直角露出
    p.rect_filled(rect, 15.0, bot);
    let inner = rect.shrink2(Vec2::new(4.0, 4.0));
    gradient(p, inner, top, bot, true);
    p.rect_filled(inner, 11.0, Color32::TRANSPARENT);

    let galley = p.layout_no_wrap(text.to_owned(), bold(16.0), Color32::WHITE);
    let tw = galley.size().x;
    let iw = if icon.is_some() { 26.0 } else { 0.0 };
    let x0 = rect.center().x - (tw + iw) * 0.5;
    if let Some(f) = icon {
        let ir = Rect::from_center_size(Pos2::new(x0 + 9.5, rect.center().y), Vec2::splat(19.0));
        f(p, ir, Color32::WHITE);
    }
    p.text(
        Pos2::new(x0 + iw, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        bold(16.0),
        Color32::WHITE,
    );
    resp
}

/// 次要按钮：白底、1px 描边、11 圆角
pub fn ghost(ui: &mut Ui, text: &str, h: f32, small: bool) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let p = ui.painter();
    let fill = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0xF7, 0xF8, 0xFA)
    } else {
        CARD
    };
    p.rect_filled(rect, if small { 9.0 } else { 11.0 }, fill);
    p.rect_stroke(
        rect,
        if small { 9.0 } else { 11.0 },
        Stroke::new(1.0, LINE),
        egui::StrokeKind::Inside,
    );
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        bold(if small { 12.0 } else { 13.0 }),
        INK2,
    );
    resp
}

/// 策略 chip：选中态是红底白字
pub fn chip(ui: &mut Ui, text: &str, on: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), bold(13.0), INK2);
    let w = galley.size().x + 26.0;
    let h = 32.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(rect, 11.0, RED);
        p.text(rect.center(), Align2::CENTER_CENTER, text, bold(13.0), Color32::WHITE);
    } else {
        p.rect_filled(rect, 11.0, CARD);
        p.rect_stroke(rect, 11.0, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
        p.text(rect.center(), Align2::CENTER_CENTER, text, bold(13.0), INK2);
    }
    resp
}

/// 分段控件：灰底槽 + 选中白块
pub fn seg(ui: &mut Ui, labels: &[&str], active: usize, wide: bool) -> Option<usize> {
    let width = if wide { ui.available_width() } else { 0.0 };
    let h = 34.0;
    let (rect, _) = if wide {
        ui.allocate_exact_size(Vec2::new(width, h), Sense::hover())
    } else {
        let mut w = 8.0;
        for l in labels {
            let g = ui.painter().layout_no_wrap(l.to_string(), bold(12.5), SUB);
            w += g.size().x + 22.0 + 2.0;
        }
        ui.allocate_exact_size(Vec2::new(w, h), Sense::hover())
    };
    let p = ui.painter();
    p.rect_filled(rect, 10.0, SEG_BG);

    let inner = rect.shrink(2.0);
    let n = labels.len() as f32;
    let cw = inner.width() / n;
    let mut hit = None;
    for (i, label) in labels.iter().enumerate() {
        let cr = Rect::from_min_size(
            Pos2::new(inner.left() + cw * i as f32, inner.top()),
            Vec2::new(cw - 2.0, inner.height()),
        );
        let resp = ui.interact(cr, ui.id().with(("seg", i, label)), Sense::click());
        if resp.clicked() {
            hit = Some(i);
        }
        if i == active {
            p.rect_filled(cr, 8.0, CARD);
            p.text(cr.center(), Align2::CENTER_CENTER, *label, bold(12.5), RED);
        } else {
            p.text(cr.center(), Align2::CENTER_CENTER, *label, bold(12.5), SUB);
        }
    }
    hit
}

/* ============================ 文本块 ============================ */

/// 一行 key-value（左侧标签 + 右侧值）
pub fn kv_row(ui: &mut Ui, label: &str, value: &str) {
    let w = ui.available_width();
    let h = 34.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let p = ui.painter();
    p.text(
        Pos2::new(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        bold(13.0),
        INK2,
    );
    p.text(
        Pos2::new(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        value,
        font(12.5),
        SUB,
    );
}

/// 虚线分隔（参数卡的行间）
pub fn dashed_line(ui: &mut Ui) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
    let p = ui.painter();
    let y = rect.center().y;
    let mut x = rect.left();
    while x < rect.right() {
        let x2 = (x + 4.0).min(rect.right());
        p.line_segment([Pos2::new(x, y), Pos2::new(x2, y)], Stroke::new(1.0, LINE));
        x += 7.0;
    }
}

/// 底部 toast
pub fn toast(ui: &mut Ui, text: &str) {
    let screen = ui.ctx().content_rect();
    let p = ui.painter();
    let galley = p.layout_no_wrap(text.to_owned(), bold(13.0), Color32::WHITE);
    let pad = Vec2::new(16.0, 9.0);
    let size = galley.size() + pad * 2.0;
    let rect = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.bottom() - 96.0 - size.y * 0.5),
        size,
    );
    p.rect_filled(rect, 11.0, Color32::from_rgba_unmultiplied(17, 21, 28, 235));
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        bold(13.0),
        Color32::WHITE,
    );
}

/* ============================ 图标 ============================ */

fn poly(p: &egui::Painter, pts: &[(f32, f32)], rect: Rect, color: Color32, fill: bool, w: f32) {
    let k = rect.width() / 24.0;
    let v: Vec<Pos2> = pts
        .iter()
        .map(|(x, y)| Pos2::new(rect.left() + x * k, rect.top() + y * k))
        .collect();
    if fill {
        p.add(Shape::convex_polygon(v, color, Stroke::NONE));
    } else if v.len() >= 2 {
        for i in 0..v.len() - 1 {
            p.line_segment([v[i], v[i + 1]], Stroke::new(w, color));
        }
    }
}

/// 顶栏 logo：深红圆角方块 + 5 根白柱 + 2 根蓝柱（与原 SVG 同坐标）
pub fn logo(p: &egui::Painter, rect: Rect) {
    let k = rect.width() / 24.0;
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * k, rect.top() + y * k);
    p.rect_filled(rect, 5.47 * k, Color32::from_rgb(0xA8, 0x0E, 0x0A));

    let bars: [(f32, f32, f32, Color32); 7] = [
        (5.064, 12.048, 4.08, Color32::from_rgb(0xFF, 0xFD, 0xFA)),
        (7.128, 9.216, 6.912, Color32::from_rgb(0xFF, 0xFD, 0xFA)),
        (9.192, 10.608, 5.52, Color32::from_rgb(0xFF, 0xFD, 0xFA)),
        (11.256, 7.2, 8.928, Color32::from_rgb(0xFF, 0xFD, 0xFA)),
        (13.32, 9.504, 6.624, Color32::from_rgb(0xFF, 0xFD, 0xFA)),
        (15.384, 11.136, 4.992, Color32::from_rgb(0x5A, 0xA0, 0xFF)),
        (17.448, 8.208, 7.92, Color32::from_rgb(0x5A, 0xA0, 0xFF)),
    ];
    for (x, y, h, c) in bars {
        let r = Rect::from_min_max(at(x, y), at(x + 1.488, y + h));
        p.rect_filled(r, 0.744 * k, c);
    }
}

/// 同步图标：圆形箭头
pub fn icon_sync(p: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width() * 0.36;
    let pts: Vec<Pos2> = (0..=20)
        .map(|i| {
            let a = -1.2 + 5.2 * (i as f32 / 20.0);
            Pos2::new(c.x + a.cos() * r, c.y + a.sin() * r)
        })
        .collect();
    for w in pts.windows(2) {
        p.line_segment([w[0], w[1]], Stroke::new(2.1, color));
    }
    // 箭头
    let tip = pts[pts.len() - 1];
    poly(
        p,
        &[
            (tip.x - rect.left(), tip.y - rect.top() - 3.2),
            (tip.x - rect.left(), tip.y - rect.top() + 3.4),
            (tip.x - rect.left() + 5.2, tip.y - rect.top() - 0.2),
        ],
        rect,
        color,
        true,
        1.0,
    );
}

/// 底栏四个图标，`kind` 0..3
pub fn tab_icon(p: &egui::Painter, rect: Rect, kind: usize, color: Color32) {
    let k = rect.width() / 24.0;
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * k, rect.top() + y * k);
    match kind {
        0 => {
            // 闪电
            poly(
                p,
                &[(13.0, 2.0), (4.5, 13.5), (11.0, 13.5), (10.0, 22.0), (19.5, 10.0), (13.0, 10.0), (13.0, 2.0)],
                rect,
                color,
                false,
                1.9 * k,
            );
        }
        1 => {
            // 票 + 勾
            let r = Rect::from_min_max(at(4.0, 3.0), at(20.0, 21.0));
            p.rect_stroke(r, 2.5 * k, Stroke::new(1.9 * k, color), egui::StrokeKind::Inside);
            p.line_segment([at(8.0, 8.0), at(16.0, 8.0)], Stroke::new(1.9 * k, color));
            p.line_segment([at(8.0, 12.0), at(13.0, 12.0)], Stroke::new(1.9 * k, color));
            let g = [at(8.0, 16.5), at(9.8, 18.3), at(13.2, 14.7)];
            for w in g.windows(2) {
                p.line_segment([w[0], w[1]], Stroke::new(2.0 * k, color));
            }
        }
        2 => {
            // 柱状
            let bars = [(4.0, 10.0, 20.0), (9.0, 4.0, 20.0), (14.0, 13.0, 20.0), (19.0, 8.0, 20.0)];
            for (x, y0, y1) in bars {
                p.line_segment([at(x, y0), at(x, y1)], Stroke::new(2.0 * k, color));
            }
        }
        _ => {
            // 列表
            for (y, x1) in [(6.0, 20.0), (12.0, 20.0), (18.0, 14.0)] {
                p.line_segment([at(4.0, y), at(x1, y)], Stroke::new(2.0 * k, color));
            }
        }
    }
}

/// 闪电（主按钮图标）
pub fn icon_bolt(p: &egui::Painter, rect: Rect, color: Color32) {
    poly(
        p,
        &[(13.0, 2.0), (4.5, 13.5), (11.0, 13.5), (10.0, 22.0), (19.5, 10.0), (13.0, 10.0)],
        rect,
        color,
        true,
        1.0,
    );
}

/// 对勾（核对按钮图标）
pub fn icon_check(p: &egui::Painter, rect: Rect, color: Color32) {
    let k = rect.width() / 24.0;
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * k, rect.top() + y * k);
    let pts = [at(4.5, 12.5), at(9.5, 17.5), at(19.5, 6.5)];
    for w in pts.windows(2) {
        p.line_segment([w[0], w[1]], Stroke::new(2.4 * k, color));
    }
}
