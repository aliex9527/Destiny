//! 离屏渲染器：把 egui 的三角形用 CPU 光栅化成图片。
//!
//! 这台开发机是 VMware 虚拟机，没有可用的 OpenGL，eframe 的窗口后端起不来，
//! 也就没法"跑起来看一眼"。但 egui 最终只是吐出一堆带纹理的三角形，
//! 自己光栅化就能得到与真机一致的画面，用来检查配色、字号、间距、对齐。
//!
//! 用法：
//!     cargo run --release --bin render
//! 产物：target/preview-{0..3}.ppm（手机逻辑尺寸 360×780，像素密度 2×）

use destiny::{app::App, install_cn_font, install_style};
use egui::{Color32, Rect};

const LOGIC_W: f32 = 360.0;
const LOGIC_H: f32 = 780.0;
const PPP: f32 = 2.0;

/// sRGB → 线性，预计算表
fn srgb_lut() -> ([f32; 256], ()) {
    let mut t = [0.0f32; 256];
    for i in 0..256 {
        let x = i as f32 / 255.0;
        t[i] = if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        };
    }
    (t, ())
}

fn linear_to_srgb(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

struct Canvas {
    w: usize,
    h: usize,
    /// 线性空间 RGB，已预乘 alpha 的高光通道不需要，直接存线性色与 a
    buf: Vec<[f32; 4]>,
    lut: [f32; 256],
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        let (lut, _) = srgb_lut();
        Self {
            w,
            h,
            buf: vec![[0.0, 0.0, 0.0, 1.0]; w * h],
            lut,
        }
    }

    fn lin(&self, c: Color32) -> [f32; 3] {
        [
            self.lut[c.r() as usize],
            self.lut[c.g() as usize],
            self.lut[c.b() as usize],
        ]
    }

    /// 源色（线性、非预乘）+ 覆盖率 alpha，over 混合到目标
    fn blend(&mut self, x: usize, y: usize, rgb: [f32; 3], a: f32) {
        if a <= 0.001 {
            return;
        }
        let d = &mut self.buf[y * self.w + x];
        let a = a.min(1.0);
        let inv = 1.0 - a;
        d[0] = rgb[0] * a + d[0] * inv;
        d[1] = rgb[1] * a + d[1] * inv;
        d[2] = rgb[2] * a + d[2] * inv;
        d[3] = a + d[3] * inv;
    }

    fn to_rgb8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.w * self.h * 3);
        for p in &self.buf {
            for c in 0..3 {
                out.push((linear_to_srgb(p[c]) * 255.0 + 0.5) as u8);
            }
        }
        out
    }
}

struct Atlas {
    w: usize,
    h: usize,
    px: Vec<Color32>,
}

impl Atlas {
    /// 双线性采样，返回 (rgb, alpha)
    fn sample(&self, u: f32, v: f32) -> (Color32, f32) {
        if self.w == 0 || self.h == 0 {
            return (Color32::WHITE, 1.0);
        }
        let x = (u * self.w as f32 - 0.5).clamp(0.0, (self.w - 1) as f32);
        let y = (v * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.w - 1);
        let y1 = (y0 + 1).min(self.h - 1);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let mut acc = [0.0f32; 4];
        let corners = [
            (x0, y0, (1.0 - fx) * (1.0 - fy)),
            (x1, y0, fx * (1.0 - fy)),
            (x0, y1, (1.0 - fx) * fy),
            (x1, y1, fx * fy),
        ];
        for (cx, cy, wgt) in corners {
            let c = self.px[cy * self.w + cx];
            acc[0] += c.r() as f32 * wgt;
            acc[1] += c.g() as f32 * wgt;
            acc[2] += c.b() as f32 * wgt;
            acc[3] += c.a() as f32 * wgt;
        }
        (
            Color32::from_rgb(acc[0] as u8, acc[1] as u8, acc[2] as u8),
            acc[3] / 255.0,
        )
    }
}

/// 把一帧的纹理增量合并进图集（字体图集通常第一帧全量、后续增量）
fn apply_deltas(atlas: &mut Atlas, delta: &egui::TexturesDelta) {
    for (_id, items) in &delta.set {
        for d in items {
            let img = match &d.image {
                egui::ImageData::Color(c) => c.clone(),
            };
            let (w, h) = (img.width(), img.height());
            match d.pos {
                None => {
                    *atlas = Atlas {
                        w,
                        h,
                        px: img.pixels.clone(),
                    };
                }
                Some([ox, oy]) => {
                    for row in 0..h {
                        for col in 0..w {
                            let (x, y) = (ox + col, oy + row);
                            if y < atlas.h && x < atlas.w {
                                atlas.px[y * atlas.w + x] = img.pixels[row * w + col];
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    use std::io::Write;
    macro_rules! step { ($($t:tt)*) => { println!($($t)*); let _ = std::io::stdout().flush(); } }
    step!("[1] 构造 App");
    let mut app = App::new(None);
    step!("[2] App 就绪，draws={}", app.draws.len());
    let ctx = egui::Context::default();
    step!("[3] 安装字体");
    install_cn_font(&ctx);
    step!("[4] 安装样式");
    install_style(&ctx);
    step!("[5] 开始渲染");

    let tabs = [
        destiny::app::Tab::Predict,
        destiny::app::Tab::Check,
        destiny::app::Tab::Analysis,
        destiny::app::Tab::History,
    ];

    // 图集跨页签共享：字体只会被加入一次，后面的页签不会再有增量
    let mut atlas = Atlas {
        w: 0,
        h: 0,
        px: Vec::new(),
    };

    for (idx, tab) in tabs.iter().enumerate() {
        app.set_tab(*tab);

        let mut last: Option<egui::FullOutput> = None;
        // egui 的字体图集是懒增长的：某帧绘制时按旧尺寸算 uv，
        // 绘制后才扩容并给出增量。必须一直跑到「某一帧不再扩容」，
        // 那一帧的 uv 才与最终图集尺寸一致，采样才不会错位。
        let mut settled = 0;
        for _ in 0..40 {
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(LOGIC_W, LOGIC_H),
            ));
            let mut vp = egui::ViewportInfo::default();
            vp.native_pixels_per_point = Some(PPP);
            raw.viewports.insert(egui::ViewportId::ROOT, vp);

            let mut out = ctx.run_ui(raw, |ui| {
                ui.set_width(LOGIC_W);
                app.draw(ui);
            });
            let grew = !out.textures_delta.set.is_empty();
            apply_deltas(&mut atlas, &out.textures_delta);
            out.textures_delta.clear();
            last = Some(out);
            if grew {
                settled = 0;
            } else {
                settled += 1;
                if settled >= 2 {
                    break;
                }
            }
        }
        let mut out = last.unwrap();

        let mut canvas = Canvas::new(
            (LOGIC_W * PPP) as usize,
            (LOGIC_H * PPP) as usize,
        );
        let ppp = out.pixels_per_point;

        let real = (0usize, 0usize);
        // 图集里若干像素的取值，用来确认覆盖率存在哪个通道
        let mut hist = [0usize; 4];
        let mut sample = Vec::new();
        for px in atlas.px.iter() {
            if px.a() > 0 { hist[3] += 1; }
            if px.r() > 0 { hist[0] += 1; }
        }
        for px in atlas.px.iter().filter(|p| p.a() > 8).take(3) {
            sample.push(format!("r{}g{}b{}a{}", px.r(), px.g(), px.b(), px.a()));
        }
        step!("[7] 图集 {}x{}，a>0:{} r>0:{}  样本 {:?}",
              atlas.w, atlas.h, hist[3], hist[0], sample);
        // 把图集的 alpha 通道导出，肉眼确认字形位置与取值
        {
            let mut pgm = Vec::new();
            pgm.extend_from_slice(format!("P5\n{} {}\n255\n", atlas.w, atlas.h).as_bytes());
            for px in &atlas.px {
                pgm.push(px.a());
            }
            let _ = std::fs::write(format!("target/atlas-{}.pgm", idx), pgm);
        }
        let shapes = std::mem::take(&mut out.shapes);
        let prims = ctx.tessellate(shapes, ppp);
        {
            let mut umax: f32 = 0.0;
            let mut vmax: f32 = 0.0;
            let mut ntext = 0usize;
            let mut ntri = 0usize;
            for pr in &prims {
                if let egui::epaint::Primitive::Mesh(m) = &pr.primitive {
                    ntri += m.indices.len() / 3;
                    for v in &m.vertices {
                        if v.uv.x > 0.0 || v.uv.y > 0.0 {
                            ntext += 1;
                        }
                        umax = umax.max(v.uv.x);
                        vmax = vmax.max(v.uv.y);
                    }
                }
            }
            step!("[8] {} 个图元 / {} 三角形 / 非零 uv 顶点 {} / max u={:.3} v={:.3}",
                  prims.len(), ntri, ntext, umax, vmax);
        }
        for prim in &prims {
            let clip = prim.clip_rect;
            if let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive {
                for tri in mesh.indices.chunks_exact(3) {
                    let v = [
                        &mesh.vertices[tri[0] as usize],
                        &mesh.vertices[tri[1] as usize],
                        &mesh.vertices[tri[2] as usize],
                    ];
                    raster(&mut canvas, &atlas, v, clip, ppp);
                }
            }
        }

        out.textures_delta.clear();

        let rgb = canvas.to_rgb8();
        let path = format!("target/preview-{}.ppm", idx);
        let mut ppm = Vec::new();
        ppm.extend_from_slice(format!("P6\n{} {}\n255\n", canvas.w, canvas.h).as_bytes());
        ppm.extend_from_slice(&rgb);
        std::fs::write(&path, ppm).unwrap();
        println!("写出 {} （{}×{}）", path, canvas.w, canvas.h);
    }
}

fn raster(
    canvas: &mut Canvas,
    atlas: &Atlas,
    v: [&egui::epaint::Vertex; 3],
    clip: Rect,
    ppp: f32,
) {
    let p: Vec<egui::Pos2> = v.iter().map(|x| x.pos * ppp).collect();
    // 屏幕空间包围盒 ∩ 裁剪框
    let minx = p.iter().map(|q| q.x).fold(f32::MAX, f32::min).max(clip.left() * ppp);
    let maxx = p.iter().map(|q| q.x).fold(f32::MIN, f32::max).min(clip.right() * ppp);
    let miny = p.iter().map(|q| q.y).fold(f32::MAX, f32::min).max(clip.top() * ppp);
    let maxy = p.iter().map(|q| q.y).fold(f32::MIN, f32::max).min(clip.bottom() * ppp);
    if maxx <= minx || maxy <= miny {
        return;
    }
    let x0 = minx.floor().max(0.0) as usize;
    let x1 = (maxx.ceil() as usize).min(canvas.w);
    let y0 = miny.floor().max(0.0) as usize;
    let y1 = (maxy.ceil() as usize).min(canvas.h);

    // 边函数
    let area = (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[2].x - p[0].x) * (p[1].y - p[0].y);
    if area.abs() < 1e-6 {
        return;
    }
    let inv_area = 1.0 / area;

    for y in y0..y1 {
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = ((p[1].x - px) * (p[2].y - py) - (p[2].x - px) * (p[1].y - py)) * inv_area;
            let w1 = ((p[2].x - px) * (p[0].y - py) - (p[0].x - px) * (p[2].y - py)) * inv_area;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let uv = v[0].uv.to_vec2() * w0 + v[1].uv.to_vec2() * w1 + v[2].uv.to_vec2() * w2;
            let (_tex, ta) = atlas.sample(uv.x, uv.y);

            let c0 = v[0].color;
            let c1 = v[1].color;
            let c2 = v[2].color;
            let cr = c0.r() as f32 * w0 + c1.r() as f32 * w1 + c2.r() as f32 * w2;
            let cg = c0.g() as f32 * w0 + c1.g() as f32 * w1 + c2.g() as f32 * w2;
            let cb = c0.b() as f32 * w0 + c1.b() as f32 * w1 + c2.b() as f32 * w2;
            let ca = (c0.a() as f32 * w0 + c1.a() as f32 * w1 + c2.a() as f32 * w2) / 255.0;

            // 字形纹理只提供覆盖率。
            // 注意必须用 sample 返回的 alpha——Color32::from_rgb 构造出的 a 恒为 255，
            // 用它会让每个字形都被填成实心方块。
            let t = if atlas.w == 0 { 1.0 } else { ta };
            let alpha = ca * t;
            if alpha <= 0.002 {
                continue;
            }
            let rgb = [
                canvas.lut[cr as usize],
                canvas.lut[cg as usize],
                canvas.lut[cb as usize],
            ];
            canvas.blend(x, y, rgb, alpha);
        }
    }
}
