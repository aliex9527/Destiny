//! 应用状态与界面
//!
//! 布局不用 egui 的 Panel（它们在 `App::ui` 这种嵌套 Ui 下语义容易混乱），
//! 改成「固定高顶栏 + 可滚动主体 + 固定底栏」的手工纵向布局，控制更直接。

use std::path::PathBuf;

use eframe::egui;
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::data::{self, Draw, BACK_PICK, BM, FRONT_PICK, FM};
use crate::engine::{self, PredictOpts, PredictResult, Stats, Strategy};
use crate::theme;
use crate::ticket;

const HEADER_H: f32 = 56.0;
const NAV_H: f32 = 62.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Predict,
    Check,
    Analysis,
    History,
}

impl Tab {
    const ALL: [(Tab, &'static str); 4] = [
        (Tab::Predict, "预测"),
        (Tab::Check, "核对"),
        (Tab::Analysis, "分析"),
        (Tab::History, "历史"),
    ];
}

pub struct App {
    pub draws: Vec<Draw>,
    pub tab: Tab,
    pub store: Option<PathBuf>,

    // 预测
    pub strategy: Strategy,
    pub window: usize,
    pub count: usize,
    pub predict: Option<PredictResult>,
    pub predict_ms: f64,

    // 核对
    pub ck_front: Vec<u8>,
    pub ck_back: Vec<u8>,
    pub ck_outcome: Option<ticket::Outcome>,
    pub ck_target: Option<Draw>,

    // 分析
    pub stats: Stats,

    // 历史
    pub hist_rows: usize,

    pub toast: Option<(String, f64)>,
    pub version: &'static str,
}

impl App {
    pub fn new(store: Option<PathBuf>) -> Self {
        // 本地存过更新数据就用本地的，否则用打包时内嵌的基线
        let mut draws = store
            .as_ref()
            .and_then(|p| data::load(p))
            .unwrap_or_else(data::baseline);
        if draws.is_empty() {
            draws = data::baseline();
        }
        let stats = engine::build_stats(&draws, 100);
        App {
            draws,
            tab: Tab::Predict,
            store,
            strategy: Strategy::Balanced,
            window: 100,
            count: 5,
            predict: None,
            predict_ms: 0.0,
            ck_front: Vec::new(),
            ck_back: Vec::new(),
            ck_outcome: None,
            ck_target: None,
            stats,
            hist_rows: 30,
            toast: None,
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn latest(&self) -> Option<&Draw> {
        self.draws.last()
    }

    fn say(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 2.6));
    }

    /* ============================ 顶栏 ============================ */

    fn header(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, HEADER_H), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 0.0, theme::CARD);
        p.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, theme::LINE),
        );

        // 方形 logo
        let logo = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, rect.center().y - 18.0),
            Vec2::splat(36.0),
        );
        p.rect_filled(logo, 10.0, theme::RED);
        p.text(
            logo.center(),
            Align2::CENTER_CENTER,
            "D",
            theme::font(18.0),
            Color32::WHITE,
        );

        p.text(
            Pos2::new(logo.right() + 10.0, rect.center().y - 9.0),
            Align2::LEFT_CENTER,
            "Destiny",
            theme::font(16.5),
            theme::INK,
        );
        let subtitle = match self.latest() {
            Some(d) => format!("第 {} 期 · {}", d.issue, short_date(&d.date)),
            None => "暂无数据".to_string(),
        };
        p.text(
            Pos2::new(logo.right() + 10.0, rect.center().y + 9.0),
            Align2::LEFT_CENTER,
            subtitle,
            theme::font(11.5),
            theme::SUB,
        );

        // 右侧：共 N 期
        p.text(
            Pos2::new(rect.right() - 14.0, rect.center().y),
            Align2::RIGHT_CENTER,
            format!("{} 期", self.draws.len()),
            theme::font(12.0),
            theme::SUB,
        );
    }

    /* ============================ 底栏 ============================ */

    fn nav(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, NAV_H), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 0.0, theme::CARD);
        p.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, theme::LINE));

        let n = Tab::ALL.len();
        let cw = w / n as f32;
        for (i, (tab, label)) in Tab::ALL.iter().enumerate() {
            let cr = Rect::from_min_size(
                Pos2::new(rect.left() + cw * i as f32, rect.top()),
                Vec2::new(cw, NAV_H),
            );
            let resp = ui.interact(cr, ui.id().with(("nav", i)), Sense::click());
            let active = self.tab == *tab;
            if resp.clicked() {
                self.tab = *tab;
            }
            let color = if active { theme::RED } else { theme::SUB };
            ui.painter().text(
                cr.center(),
                Align2::CENTER_CENTER,
                *label,
                theme::font(14.0),
                color,
            );
            if active {
                let bar = Rect::from_min_size(
                    Pos2::new(cr.center().x - 11.0, cr.top() + 4.0),
                    Vec2::new(22.0, 3.0),
                );
                ui.painter().rect_filled(bar, 2.0, theme::RED);
            }
        }
    }

    /* ============================ 主体 ============================ */

    pub fn body(&mut self, ui: &mut egui::Ui) {
        match self.tab {
            Tab::Predict => self.page_predict(ui),
            Tab::Check => self.page_check(ui),
            Tab::Analysis => self.page_analysis(ui),
            Tab::History => self.page_history(ui),
        }
    }

    /* ---------------- 预测 ---------------- */

    fn page_predict(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);

        theme::card(ui, |ui| {
            theme::note(ui, "选号策略", theme::INK, 14.0);
            ui.add_space(9.0);
            let mut picked = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(7.0, 7.0);
                for s in Strategy::ALL {
                    if theme::chip(ui, s.name(), self.strategy == s).clicked() {
                        picked = Some(s);
                    }
                }
            });
            if let Some(s) = picked {
                self.strategy = s;
                self.predict = None;
            }
            ui.add_space(8.0);
            theme::note(ui, self.strategy.desc(), theme::SUB, 11.5);
        });

        ui.add_space(10.0);

        theme::card(ui, |ui| {
            theme::note(ui, "统计窗口", theme::INK, 14.0);
            ui.add_space(9.0);
            let mut new_window = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(7.0, 7.0);
                for (label, v) in [("近30期", 30usize), ("近50期", 50), ("近100期", 100), ("全部", 0)] {
                    if theme::chip(ui, label, self.window == v).clicked() {
                        new_window = Some(v);
                    }
                }
            });
            if let Some(v) = new_window {
                self.window = v;
                self.predict = None;
            }

            ui.add_space(12.0);
            theme::note(ui, "生成注数", theme::INK, 14.0);
            ui.add_space(9.0);
            let mut new_count = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(7.0, 7.0);
                for v in [1usize, 3, 5, 10] {
                    if theme::chip(ui, &format!("{} 注", v), self.count == v).clicked() {
                        new_count = Some(v);
                    }
                }
            });
            if let Some(v) = new_count {
                self.count = v;
                self.predict = None;
            }
        });

        ui.add_space(14.0);
        if theme::primary_button(ui, "生成号码").clicked() {
            let t0 = std::time::Instant::now();
            let opts = PredictOpts {
                strategy: self.strategy,
                window: self.window,
                count: self.count,
                seed: 0,
                ..Default::default()
            };
            let res = engine::predict(&self.draws, &opts);
            self.predict_ms = t0.elapsed().as_secs_f64() * 1000.0;
            let n = res.sets.len();
            self.predict = Some(res);
            self.say(format!("已生成 {} 注，用时 {:.0} ms", n, self.predict_ms));
        }

        ui.add_space(14.0);

        if let Some(res) = &self.predict {
            // 先把要显示的数据摘出来，避免与 self 的可变借用冲突
            let sets: Vec<_> = res
                .sets
                .iter()
                .map(|s| (s.rank, s.f, s.b, s.score, s.tags.clone(), s.sum, s.odd_f, s.big_f, s.consec))
                .collect();
            let meta = (res.window, res.strategy.name(), res.sets.len());
            for (rank, f, b, score, tags, sum, odd, big, consec) in sets {
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::note(ui, &format!("第 {} 注", rank), theme::INK, 14.5);
                        ui.add_space(6.0);
                        if rank == 1 {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(46.0, 20.0), Sense::hover());
                            ui.painter().rect_filled(r, 6.0, theme::RED_SOFT);
                            ui.painter().text(
                                r.center(),
                                Align2::CENTER_CENTER,
                                "首选",
                                theme::font(11.0),
                                theme::RED,
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            theme::note(ui, &format!("{:.3}", score), theme::GOLD, 13.0);
                        });
                    });
                    ui.add_space(9.0);
                    theme::balls_row(ui, &f, &b, 16.5, None);
                    ui.add_space(9.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 5.0);
                        for t in &tags {
                            tag_pill(ui, t);
                        }
                        let _ = (sum, odd, big, consec);
                    });
                });
                ui.add_space(9.0);
            }
            theme::note(
                ui,
                &format!(
                    "策略 {} · 窗口 {} 期 · 共 {} 注 · 已排除历史开奖组合",
                    meta.1,
                    if meta.0 == 0 { self.draws.len() } else { meta.0 },
                    meta.2
                ),
                theme::SUB,
                11.5,
            );
            ui.add_space(20.0);
        }
    }

    /* ---------------- 核对 ---------------- */

    fn page_check(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);

        let target = self.ck_target.clone().or_else(|| self.latest().cloned());
        if let Some(t) = &target {
            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::note(ui, &format!("第 {} 期开奖", t.issue), theme::INK, 14.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        theme::note(ui, &t.date, theme::SUB, 11.5);
                    });
                });
                ui.add_space(9.0);
                theme::balls_row(ui, &t.f, &t.b, 15.0, None);
            });
        }
        ui.add_space(10.0);

        // 选号网格
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                theme::note(ui, "前区", theme::INK, 14.0);
                theme::note(ui, &format!("已选 {}/5", self.ck_front.len()), theme::SUB, 11.5);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::ghost_button(ui, "清空", Some(56.0)).clicked() {
                        self.ck_front.clear();
                        self.ck_back.clear();
                        self.ck_outcome = None;
                    }
                });
            });
            ui.add_space(9.0);
            number_grid(ui, FM as u8, &mut self.ck_front, FRONT_PICK, theme::RED, theme::RED_SOFT);

            ui.add_space(13.0);
            ui.horizontal(|ui| {
                theme::note(ui, "后区", theme::INK, 14.0);
                theme::note(ui, &format!("已选 {}/2", self.ck_back.len()), theme::SUB, 11.5);
            });
            ui.add_space(9.0);
            number_grid(ui, BM as u8, &mut self.ck_back, BACK_PICK, theme::BLUE, theme::BLUE_SOFT);
        });

        ui.add_space(14.0);
        let ready = self.ck_front.len() == FRONT_PICK && self.ck_back.len() == BACK_PICK;
        let resp = theme::primary_button(ui, if ready { "核对" } else { "请选择 5 + 2 个号码" });
        if resp.clicked() && ready {
            if let Some(t) = &target {
                let mut f = [0u8; 5];
                f.copy_from_slice(&self.ck_front);
                let mut b = [0u8; 2];
                b.copy_from_slice(&self.ck_back);
                f.sort_unstable();
                b.sort_unstable();
                self.ck_outcome = Some(ticket::check(&f, &b, t));
            }
        }

        ui.add_space(14.0);
        if let Some(o) = &self.ck_outcome {
            let win = o.is_win();
            let level_name = o.name();
            let hit_text = o.hit_text();
            let amount = o.amount();
            let f_mask = o.front_mask;
            let b_mask = o.back_mask;
            let (f, b) = (self.ck_front.clone(), self.ck_back.clone());

            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::note(
                        ui,
                        if win { "中奖" } else { "未中奖" },
                        if win { theme::RED } else { theme::SUB },
                        16.0,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        theme::note(ui, level_name, if win { theme::RED } else { theme::SUB }, 14.0);
                    });
                });
                ui.add_space(10.0);
                let mut mask = f_mask.to_vec();
                mask.extend_from_slice(&b_mask);
                theme::balls_row(ui, &f, &b, 15.0, Some(&mask));
                ui.add_space(10.0);
                theme::note(ui, &format!("命中 前区 {} + 后区 {}", hit_text.split(" + ").next().unwrap_or("0"), hit_text.split(" + ").nth(1).unwrap_or("0")), theme::INK2, 13.0);
                if let Some(amt) = amount {
                    ui.add_space(4.0);
                    theme::note(ui, &format!("固定奖金 ¥{}", amt), theme::RED, 15.0);
                } else if win {
                    ui.add_space(4.0);
                    theme::note(ui, "浮动奖金，以官方公告为准", theme::SUB, 11.5);
                }
            });
            ui.add_space(20.0);
        }
    }

    /* ---------------- 分析 ---------------- */

    fn page_analysis(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);

        let st = &self.stats;
        let (n, sum_mean, span_mean, q10, q90) =
            (st.n, st.sum_mean, st.span_mean, st.sum_q10, st.sum_q90);

        theme::card(ui, |ui| {
            theme::note(ui, "近 100 期概览", theme::INK, 14.0);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                stat_block(ui, "样本", &format!("{}", n), theme::INK);
                stat_block(ui, "和值均值", &format!("{:.1}", sum_mean), theme::INK);
                stat_block(ui, "跨度均值", &format!("{:.1}", span_mean), theme::INK);
            });
            ui.add_space(10.0);
            theme::note(
                ui,
                &format!("和值集中区间 {:.0} ~ {:.0}（10%~90% 分位）", q10, q90),
                theme::SUB,
                11.5,
            );
        });

        ui.add_space(10.0);

        // 热号 TOP
        let mut freq: Vec<(usize, u32)> = (1..=FM).map(|v| (v, st.freq_f[v])).collect();
        freq.sort_by(|a, b| b.1.cmp(&a.1));
        let top: Vec<(usize, u32)> = freq.iter().take(8).cloned().collect();
        let max_freq = top.first().map(|x| x.1).unwrap_or(1).max(1);

        theme::card(ui, |ui| {
            theme::note(ui, "热号排行", theme::INK, 14.0);
            ui.add_space(10.0);
            for (num, c) in &top {
                bar_row(ui, *num as u8, *c, max_freq, theme::RED, "次");
            }
        });

        ui.add_space(10.0);

        // 遗漏排行
        let mut omit: Vec<(usize, usize)> = (1..=FM).map(|v| (v, st.omit_f[v])).collect();
        omit.sort_by(|a, b| b.1.cmp(&a.1));
        let top_omit: Vec<(usize, usize)> = omit.iter().take(8).cloned().collect();
        let max_omit = top_omit.first().map(|x| x.1).unwrap_or(1).max(1);

        theme::card(ui, |ui| {
            theme::note(ui, "遗漏排行", theme::INK, 14.0);
            ui.add_space(10.0);
            for (num, c) in &top_omit {
                bar_row(ui, *num as u8, *c as u32, max_omit as u32, theme::BLUE, "期");
            }
        });

        ui.add_space(10.0);

        // 和值分布
        let sums = st.sums_sorted.clone();
        theme::card(ui, |ui| {
            theme::note(ui, "和值分布", theme::INK, 14.0);
            ui.add_space(10.0);
            histogram(ui, &sums);
        });
        ui.add_space(20.0);
    }

    /* ---------------- 历史 ---------------- */

    fn page_history(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        let total = self.draws.len();
        let take = self.hist_rows.min(total);
        let rows: Vec<(String, String, [u8; 5], [u8; 2])> = self
            .draws
            .iter()
            .rev()
            .take(take)
            .map(|d| (d.issue.clone(), d.date.clone(), d.f, d.b))
            .collect();

        for (issue, date, f, b) in rows {
            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::note(ui, &format!("第 {} 期", issue), theme::INK, 13.5);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        theme::note(ui, &date, theme::SUB, 11.0);
                    });
                });
                ui.add_space(7.0);
                theme::balls_row(ui, &f, &b, 13.0, None);
            });
            ui.add_space(7.0);
        }

        if take < total {
            ui.add_space(4.0);
            if theme::ghost_button(ui, &format!("加载更多（还有 {} 期）", total - take), None).clicked() {
                self.hist_rows += 30;
            }
        }
        ui.add_space(20.0);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let bg_rect = ui.max_rect();
        ui.painter().rect_filled(bg_rect, 0.0, theme::BG);

        ui.vertical(|ui| {
            self.header(ui);
            let scroll_h = (ui.available_height() - NAV_H).max(60.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(14, 0))
                        .show(ui, |ui| {
                            self.body(ui);
                        });
                });
            self.nav(ui);
        });

        // toast 淡出
        if let Some((msg, left)) = &mut self.toast {
            let dt = ui.ctx().input(|i| i.stable_dt).min(0.1) as f64;
            *left -= dt;
            let text = msg.clone();
            let alpha = (*left / 0.6).clamp(0.0, 1.0);
            let screen = ui.ctx().content_rect();
            let pos = Pos2::new(screen.center().x, screen.bottom() - NAV_H - 34.0);
            let galley = ui
                .painter()
                .layout_no_wrap(text, theme::font(13.0), Color32::WHITE);
            let size = galley.size() + Vec2::new(26.0, 16.0);
            let r = Rect::from_center_size(pos, size);
            ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("toast"),
            ))
            .rect_filled(r, 10.0, Color32::from_rgba_unmultiplied(0x1F, 0x24, 0x2C, (220.0 * alpha) as u8));
            ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("toast"),
            ))
            .galley(r.center() - size / 2.0 + Vec2::new(13.0, 8.0), galley, Color32::WHITE);
            if *left <= 0.0 {
                self.toast = None;
            } else {
                ui.ctx().request_repaint();
            }
        }
    }
}

/* ============================ 小组件 ============================ */

fn short_date(d: &str) -> String {
    // 2026-10-28 → 10-28
    if d.len() >= 10 {
        d[5..10].to_string()
    } else {
        d.to_string()
    }
}

fn tag_pill(ui: &mut egui::Ui, text: &str) {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::font(11.0), theme::INK2);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x + 14.0, 20.0), Sense::hover());
    ui.painter().rect_filled(rect, 6.0, theme::BG);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, theme::INK2);
}

/// 1..max 的号码选择网格，每行 7 个
fn number_grid(ui: &mut egui::Ui, max: u8, sel: &mut Vec<u8>, limit: usize, active: Color32, active_bg: Color32) {
    let cols = 7;
    let gap = 6.0;
    let w = ui.available_width();
    let cell = ((w - gap * (cols as f32 - 1.0)) / cols as f32).min(44.0);
    let rows = (max as f32 / cols as f32).ceil() as usize;

    for r in 0..rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(gap, gap);
            for c in 0..cols {
                let n = (r * cols + c + 1) as u8;
                if n > max {
                    break;
                }
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(cell), Sense::click());
                let on = sel.contains(&n);
                let bg = if on { active_bg } else { theme::BG };
                let fg = if on { active } else { theme::INK2 };
                ui.painter().rect_filled(rect, cell * 0.28, bg);
                if !on {
                    ui.painter().rect_stroke(
                        rect,
                        cell * 0.28,
                        Stroke::new(1.0, theme::LINE),
                        egui::StrokeKind::Inside,
                    );
                }
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    format!("{:02}", n),
                    theme::font(cell * 0.38),
                    fg,
                );
                if resp.clicked() {
                    if on {
                        sel.retain(|&v| v != n);
                    } else if sel.len() < limit {
                        sel.push(n);
                        sel.sort_unstable();
                    }
                }
            }
        });
        ui.add_space(gap);
    }
}

fn stat_block(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    let w = ui.available_width() / 3.0;
    ui.allocate_ui_with_layout(
        Vec2::new(w, 44.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            theme::note(ui, value, color, 18.0);
            theme::note(ui, label, theme::SUB, 11.0);
        },
    );
}

fn bar_row(ui: &mut egui::Ui, num: u8, value: u32, max: u32, color: Color32, unit: &str) {
    let w = ui.available_width();
    let h = 22.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let p = ui.painter();

    // 号码
    p.text(
        Pos2::new(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        format!("{:02}", num),
        theme::font(12.5),
        theme::INK,
    );

    // 进度条
    let bar_left = rect.left() + 34.0;
    let bar_right = rect.right() - 52.0;
    let bar = Rect::from_min_max(
        Pos2::new(bar_left, rect.center().y - 4.0),
        Pos2::new(bar_right, rect.center().y + 4.0),
    );
    p.rect_filled(bar, 4.0, theme::BG);
    let ratio = if max > 0 { value as f32 / max as f32 } else { 0.0 };
    let fill = Rect::from_min_max(
        bar.min,
        Pos2::new(bar_left + (bar_right - bar_left) * ratio.clamp(0.0, 1.0), bar.max.y),
    );
    p.rect_filled(fill, 4.0, color.gamma_multiply(0.85));

    // 数值
    p.text(
        Pos2::new(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        format!("{} {}", value, unit),
        theme::font(12.0),
        theme::SUB,
    );
}

/// 和值直方图
fn histogram(ui: &mut egui::Ui, sums: &[f64]) {
    if sums.is_empty() {
        theme::note(ui, "暂无数据", theme::SUB, 12.0);
        return;
    }
    let w = ui.available_width();
    let h = 120.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let p = ui.painter();

    let lo = sums.first().copied().unwrap_or(0.0);
    let hi = sums.last().copied().unwrap_or(0.0);
    let bins = 22usize;
    let step = ((hi - lo) / bins as f64).max(1.0);
    let mut counts = vec![0u32; bins];
    for &s in sums {
        let idx = (((s - lo) / step).floor() as usize).min(bins - 1);
        counts[idx] += 1;
    }
    let max_c = counts.iter().copied().max().unwrap_or(1).max(1);

    let plot = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + 6.0),
        Pos2::new(rect.right(), rect.bottom() - 20.0),
    );
    let bw = plot.width() / bins as f32;
    for (i, &c) in counts.iter().enumerate() {
        let bh = plot.height() * (c as f32 / max_c as f32);
        let bar = Rect::from_min_max(
            Pos2::new(plot.left() + bw * i as f32 + 1.0, plot.bottom() - bh),
            Pos2::new(plot.left() + bw * (i as f32 + 1.0) - 1.0, plot.bottom()),
        );
        p.rect_filled(bar, 2.0, theme::RED.gamma_multiply(0.55 + 0.4 * (c as f32 / max_c as f32)));
    }
    p.line_segment(
        [plot.left_bottom(), plot.right_bottom()],
        Stroke::new(1.0, theme::LINE),
    );

    p.text(
        Pos2::new(rect.left(), rect.bottom()),
        Align2::LEFT_BOTTOM,
        format!("{:.0}", lo),
        theme::font(10.5),
        theme::SUB,
    );
    p.text(
        Pos2::new(rect.right(), rect.bottom()),
        Align2::RIGHT_BOTTOM,
        format!("{:.0}", hi),
        theme::font(10.5),
        theme::SUB,
    );
}
