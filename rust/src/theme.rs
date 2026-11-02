//! 视觉层：配色、字体、号码球、卡片、按钮、图标、图表元件
//!
//! 所有数值都直接取自 web 版 `app.css`，单位是 CSS px，与 egui 的点在
//! Android 上基本 1:1（dp）。改这里的常量等于改 `app.css`，两版观感一致。

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
/// 分段控件的槽。原版 #F2F3F6 与页面背景 #F4F5F7 几乎同色，
/// 放在背景上时分不出边界，这里压深两档保证选中块有依托。
pub const SEG_BG: Color32 = Color32::from_rgb(0xEC, 0xEE, 0xF2);
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

/// 顶部安全区高度：避让安卓前置摄像头（打孔/刘海）与系统状态栏标头。
/// NativeActivity 默认全屏沉浸式，系统栏直接压在界面最顶，不预留会让
/// 标题和同步按钮被信号/时间/挖孔遮住。取 32≈大屏机型的状态栏 + 打孔区域高度。
pub const STATUS_BAR_H: f32 = 32.0;
/// 号码球直径（CSS .ball 33px）
pub const BALL_D: f32 = 33.0;

/* ============================ 字体 ============================ */

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

/// 粗体：走单独的粗体字体家族。
/// egui 不支持合成加粗，只有真正嵌入一份 Bold 字重，
/// CSS 里的 font-weight:700/750/800 才不会塌成普通字重。
pub fn bold(size: f32) -> FontId {
    FontId::new(size, crate::bold_family())
}

/// 粗体文本，供 `ui.label()` 使用。
/// egui 的 RichText 默认走 Proportional（常规字重），必须显式指定家族才是粗体。
pub fn rb(text: impl Into<String>, size: f32, color: Color32) -> egui::RichText {
    egui::RichText::new(text)
        .size(size)
        .color(color)
        .family(crate::bold_family())
}

/* ============================ 基础形状 ============================ */

/// 线性渐变矩形。EGUI 没有原生渐变，用两个三角形拼。
pub fn gradient(p: &egui::Painter, rect: Rect, from: Color32, to: Color32, vertical: bool) {
    let mut mesh = egui::Mesh::default();
    let (lt, rt, rb, lb) = if vertical {
        (from, from, to, to)
    } else {
        (from, to, to, from)
    };
    mesh.colored_vertex(rect.left_top(), lt);
    mesh.colored_vertex(rect.right_top(), rt);
    mesh.colored_vertex(rect.right_bottom(), rb);
    mesh.colored_vertex(rect.left_bottom(), lb);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(Shape::mesh(mesh));
}

/// 圆角矩形的垂直渐变。
///
/// `gradient()` 画的是直角矩形，铺到圆角控件上会把四个角切成方的。
/// 这里按 y 逐段收缩左右边界，形状就是真正的圆角矩形。
pub fn rounded_gradient(
    p: &egui::Painter,
    rect: Rect,
    radius: f32,
    from: Color32,
    to: Color32,
) {
    let r = radius
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    const N: usize = 24;
    let mut mesh = egui::Mesh::default();
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let y = rect.top() + rect.height() * t;
        let dy_top = y - rect.top();
        let dy_bot = rect.bottom() - y;
        let fy = if dy_top < r {
            r - dy_top
        } else if dy_bot < r {
            r - dy_bot
        } else {
            0.0
        };
        let inset = if fy > 0.0 {
            r - (r * r - fy * fy).max(0.0).sqrt()
        } else {
            0.0
        };
        let col = from.lerp_to_gamma(to, t);
        mesh.colored_vertex(Pos2::new(rect.left() + inset, y), col);
        mesh.colored_vertex(Pos2::new(rect.right() - inset, y), col);
    }
    for i in 0..N as u32 {
        let a = i * 2;
        mesh.add_triangle(a, a + 1, a + 3);
        mesh.add_triangle(a, a + 3, a + 2);
    }
    p.add(Shape::mesh(mesh));
}

/// 水平条：底槽 + 填充，用于柱状图与遗漏榜
pub fn hbar(p: &egui::Painter, rect: Rect, ratio: f32, from: Color32, to: Color32) {
    p.rect_filled(rect, rect.height() * 0.5, SEG_BG);
    if ratio <= 0.001 {
        return;
    }
    let w = (rect.width() * ratio.clamp(0.0, 1.0)).max(rect.height());
    let fill = Rect::from_min_size(rect.min, Vec2::new(w, rect.height()));
    // 横向渐变的圆角条：按 x 分段收缩上下边界，两端才是圆的
    let r = (rect.height() * 0.5).min(w * 0.5);
    const N: usize = 24;
    let mut mesh = egui::Mesh::default();
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let x = fill.left() + w * t;
        let dx_l = x - fill.left();
        let dx_r = fill.right() - x;
        let fx = if dx_l < r {
            r - dx_l
        } else if dx_r < r {
            r - dx_r
        } else {
            0.0
        };
        let inset = if fx > 0.0 {
            r - (r * r - fx * fx).max(0.0).sqrt()
        } else {
            0.0
        };
        let col = from.lerp_to_gamma(to, t);
        mesh.colored_vertex(Pos2::new(x, fill.top() + inset), col);
        mesh.colored_vertex(Pos2::new(x, fill.bottom() - inset), col);
    }
    for i in 0..N as u32 {
        let a = i * 2;
        mesh.add_triangle(a, a + 1, a + 3);
        mesh.add_triangle(a, a + 3, a + 2);
    }
    p.add(Shape::mesh(mesh));
}

/* ============================ 色彩空间 ============================ */

/// HSL → sRGB。原版热力格用 `hsl(4,72%,96-t*52%)` 这类色阶，
/// 用 HSL 等比降亮度比拿两个端点插值更接近原色，中间段不会偏色。
pub fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let h = h.rem_euclid(360.0) / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    if s <= f32::EPSILON {
        let v = (l * 255.0).round() as u8;
        return Color32::from_rgb(v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let f = |mut t: f32| -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(
        to8(f(h + 1.0 / 3.0)),
        to8(f(h)),
        to8(f(h - 1.0 / 3.0)),
    )
}

/* ============================ 号码球 ============================ */

/// 画一个号码球。`front` 决定红/蓝，`dim` 用于未命中。
///
/// 渐变严格照 CSS：`radial-gradient(circle at 34% 26%, HI, MID 60%, LO)`。
/// `circle` 未指定尺寸时默认 farthest-corner，半径 = 到最远角的距离 ≈ 1.983r，
/// 渐变中心相对球心偏移 (-0.32r, -0.48r)。这里用环形网格按这个色场逐点求色，
/// 比叠若干同心圆平滑得多（叠圆会出现一圈圈色带）。
pub fn draw_ball(p: &egui::Painter, center: Pos2, d: f32, n: u8, front: bool, dim: bool) {
    let r = d * 0.5;

    let (hi, mid, lo, txt) = if dim {
        let g = Color32::from_rgb(0xDF, 0xE2, 0xE8);
        (g, g, g, FAINT)
    } else if front {
        (BALL_F_HI, RED, BALL_F_LO, Color32::WHITE)
    } else {
        (BALL_B_HI, BLUE, BALL_B_LO, Color32::WHITE)
    };

    if dim {
        // 未命中：纯色浅灰球，不用渐变，视觉上自然退到背景里
        p.circle_filled(center, r, Color32::from_rgb(0xEE, 0xF0, 0xF4));
    } else {
        let focus = center + Vec2::new(-r * 0.32, -r * 0.48);
        let far = r * 1.983;

        const RINGS: usize = 10;
        const SEGS: usize = 34;
        let mut mesh = egui::Mesh::default();
        for i in 0..=RINGS {
            let t = i as f32 / RINGS as f32;
            let rad = r * t;
            for j in 0..=SEGS {
                let a = std::f32::consts::TAU * j as f32 / SEGS as f32;
                let pt = center + Vec2::new(a.cos(), a.sin()) * rad;
                let u = ((pt - focus).length() / far).clamp(0.0, 1.0);
                let col = if u < 0.6 {
                    hi.lerp_to_gamma(mid, u / 0.6)
                } else {
                    mid.lerp_to_gamma(lo, (u - 0.6) / 0.4)
                };
                mesh.colored_vertex(pt, col);
            }
        }
        let stride = (SEGS + 1) as u32;
        for i in 0..RINGS as u32 {
            for j in 0..SEGS as u32 {
                let a = i * stride + j;
                let b = a + 1;
                let c = a + stride;
                mesh.add_triangle(a, b, c + 1);
                mesh.add_triangle(a, c + 1, c);
            }
        }
        p.add(Shape::mesh(mesh));

        // 边缘柔化：切线一圈半透明同色，压掉网格的锯齿
        let edge = mid.lerp_to_gamma(lo, 0.5);
        p.circle_stroke(center, r - 0.4, Stroke::new(0.9, edge.gamma_multiply(0.55)));
    }

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
    // CSS --shadow: 0 1px 2px rgba(15,19,25,.04), 0 6px 18px -12px rgba(15,19,25,.14)
    // egui 的 Shadow 只有一层，这里取合成后的观感：几乎不下移、较大模糊、极淡。
    let shadow = egui::epaint::Shadow {
        offset: [0, 3],
        blur: 14,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(15, 19, 25, 26),
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CARD_R)
        .inner_margin(egui::Margin::symmetric(14, 14))
        .shadow(shadow)
        .show(ui, |ui| {
            ui.set_width(w - 28.0);
            add(ui);
        });
    ui.add_space(12.0);
}

/// 柔和投影：多层半透明圆角矩形叠出模糊边，替代 egui 没有的多重 box-shadow。
/// `dy` 是整体下移量，`spread` 是扩散范围。
pub fn drop_shadow(
    p: &egui::Painter,
    rect: Rect,
    rounding: f32,
    dy: f32,
    spread: f32,
    color: Color32,
) {
    const LAYERS: usize = 7;
    for i in 0..LAYERS {
        let t = i as f32 / (LAYERS - 1) as f32; // 0 → 1，越大越外扩越淡
        let grow = spread * t;
        let a = color.a() as f32 * (1.0 - t).powf(1.8) * 0.5;
        let r = Rect::from_center_size(
            rect.center() + Vec2::new(0.0, dy),
            rect.size() + Vec2::splat(grow * 2.0),
        );
        p.rect_filled(
            r,
            rounding + grow,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a as u8),
        );
    }
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
        // 默认 item_spacing.x=8 会让 add_space(4) 实占 12，左右就不对称了
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 4.0);
            para(ui, text, 11.0, FAINT, 1.65);
        });
        ui.add_space(4.0);
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
    // CSS: box-shadow 0 10px 22px -12px rgba(203,33,25,.9)
    // spread 为负值，实际是很小的一圈柔光，不能用大扩散
    drop_shadow(
        p,
        rect,
        15.0,
        if resp.is_pointer_button_down_on() { 3.0 } else { 5.0 },
        2.0,
        Color32::from_rgb(0xCB, 0x21, 0x19).gamma_multiply(0.34),
    );
    p.rect_filled(rect, 15.0, bot);
    // 圆角渐变覆盖，边角不会被切成直角
    rounded_gradient(p, rect.shrink(0.5), 14.7, top, bot);

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
        // CSS: box-shadow 0 4px 12px -6px rgba(222,51,43,.8)
        drop_shadow(p, rect, 11.0, 3.0, 2.5, RED.gamma_multiply(0.5));
        p.rect_filled(rect, 11.0, RED);
        p.text(rect.center(), Align2::CENTER_CENTER, text, bold(13.0), Color32::WHITE);
    } else {
        p.rect_filled(rect, 11.0, CARD);
        p.rect_stroke(rect, 11.0, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
        p.text(rect.center(), Align2::CENTER_CENTER, text, bold(13.0), INK2);
    }
    resp
}

/// 分段控件：灰底槽 + 选中白块。`wide` 时铺满可用宽度。
pub fn seg(ui: &mut Ui, labels: &[&str], active: usize, wide: bool) -> Option<usize> {
    let w = if wide {
        ui.available_width()
    } else {
        let mut w = 8.0;
        for l in labels {
            let g = ui.painter().layout_no_wrap(l.to_string(), bold(12.5), SUB);
            w += g.size().x + 22.0 + 2.0;
        }
        w
    };
    seg_fixed(ui, labels, active, w)
}

/// 指定宽度的分段控件（用于两个等分并排的场景）
pub fn seg_fixed(ui: &mut Ui, labels: &[&str], active: usize, width: f32) -> Option<usize> {
    let h = 34.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
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
            drop_shadow(p, cr, 8.0, 1.0, 1.5, Color32::from_rgba_unmultiplied(15, 19, 25, 40));
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
