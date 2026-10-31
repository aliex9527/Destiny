//! 界面：严格对齐 web 版 `index.html` + `app.css`
//!
//! 布局是「固定顶栏 + 同步条 + 滚动主体 + 固定底栏」，与浏览器里那套
//! flex 纵向布局一一对应。每个页签的内容顺序、文案、字号都取自原版，
//! 换技术栈不换观感。

use std::path::PathBuf;

use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::data::{self, Draw, BACK_PICK, BM, FRONT_PICK, FM};
use crate::engine::{self, PredictOpts, PredictResult, Stats, Strategy};
use crate::theme;
use crate::ticket;

/* ============================ 页签 ============================ */

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Predict,
    Check,
    Analysis,
    History,
}

impl Tab {
    /// 标签与原版底栏一致：推演 / 验票 / 分析 / 历史
    const ALL: [(Tab, &'static str); 4] = [
        (Tab::Predict, "推演"),
        (Tab::Check, "验票"),
        (Tab::Analysis, "分析"),
        (Tab::History, "历史"),
    ];
}

/// 同步状态灯
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dot {
    Ok,
    Warn,
    Bad,
    Busy,
}

impl Dot {
    fn color(self) -> Color32 {
        match self {
            Dot::Ok => theme::GREEN,
            Dot::Warn => theme::GOLD,
            Dot::Bad => theme::RED,
            Dot::Busy => theme::BLUE,
        }
    }
}

pub struct App {
    pub draws: Vec<Draw>,
    pub tab: Tab,
    pub store: Option<PathBuf>,

    /* 推演 */
    pub strategy: Strategy,
    pub window: usize,
    pub count: usize,
    pub predict: Option<PredictResult>,
    pub predict_ms: f64,

    /* 验票 */
    pub ck_front: Vec<u8>,
    pub ck_back: Vec<u8>,
    pub ck_multi: bool,
    pub ck_extra: bool,
    pub ck_target: usize,
    pub ck_outcome: Option<ticket::Outcome>,

    /* 分析 / 历史 */
    pub stats: Stats,
    pub hist_rows: usize,
    pub query: String,

    /* 通用 */
    pub toast: Option<(String, f64)>,
    pub version: &'static str,
}

impl App {
    pub fn new(store: Option<PathBuf>) -> Self {
        let mut draws = store
            .as_ref()
            .and_then(|p| data::load(p))
            .unwrap_or_else(data::baseline);
        if draws.is_empty() {
            draws = data::baseline();
        }
        let stats = engine::build_stats(&draws, 100);
        let last = draws.len().saturating_sub(1);
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
            ck_multi: false,
            ck_extra: false,
            ck_target: last,
            ck_outcome: None,
            stats,
            hist_rows: 30,
            query: String::new(),
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
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, theme::HEADER_H), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, theme::CARD);
        p.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, theme::LINE),
        );

        // 36×36 的柱状 logo
        let logo = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, rect.center().y - 18.0),
            Vec2::splat(36.0),
        );
        theme::logo(&p, logo);

        p.text(
            Pos2::new(logo.right() + 10.0, rect.center().y - 9.0),
            Align2::LEFT_CENTER,
            "Destiny",
            theme::bold(16.5),
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

        // 右侧同步按钮
        let bw = 74.0;
        let bh = 32.0;
        let btn = Rect::from_min_size(
            Pos2::new(rect.right() - 14.0 - bw, rect.center().y - bh * 0.5),
            Vec2::new(bw, bh),
        );
        let resp = ui.interact(btn, ui.id().with("sync-btn"), Sense::click());
        let fill = if resp.is_pointer_button_down_on() {
            Color32::from_rgb(0xF7, 0xF8, 0xFA)
        } else {
            theme::CARD
        };
        p.rect_filled(btn, 11.0, fill);
        p.rect_stroke(
            btn,
            11.0,
            Stroke::new(1.0, theme::LINE),
            egui::StrokeKind::Inside,
        );
        let icon = Rect::from_center_size(
            Pos2::new(btn.left() + 18.0, btn.center().y),
            Vec2::splat(15.0),
        );
        theme::icon_sync(&p, icon, theme::INK2);
        p.text(
            Pos2::new(icon.right() + 5.0, btn.center().y),
            Align2::LEFT_CENTER,
            "同步",
            theme::bold(13.0),
            theme::INK2,
        );
        if resp.clicked() {
            self.say("已是最新数据");
        }
    }

    /* ============================ 同步条 ============================ */

    fn sync_bar(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, theme::SYNC_H), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, theme::CARD);
        p.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, theme::LINE),
        );

        // 本地数据永远可用，所以状态恒为「已是最新」，只是文案里带期号
        let (dot, text) = match self.latest() {
            Some(d) => (
                Dot::Ok,
                format!("已是最新 · 第 {} 期 · {}", d.issue, d.date),
            ),
            None => (Dot::Warn, "暂无开奖数据".to_string()),
        };
        let cy = rect.center().y;
        p.circle_filled(Pos2::new(rect.left() + 17.5, cy), 3.5, dot.color());
        p.text(
            Pos2::new(rect.left() + 28.0, cy),
            Align2::LEFT_CENTER,
            text,
            theme::font(11.5),
            theme::SUB,
        );
    }

    /* ============================ 底栏 ============================ */

    fn tabbar(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, theme::TABBAR_H), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, theme::CARD);
        p.line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, theme::LINE),
        );

        let n = Tab::ALL.len();
        let cw = w / n as f32;
        for (i, (tab, label)) in Tab::ALL.iter().enumerate() {
            let cr = Rect::from_min_size(
                Pos2::new(rect.left() + cw * i as f32, rect.top()),
                Vec2::new(cw, theme::TABBAR_H),
            );
            let resp = ui.interact(cr, ui.id().with(("tab", i)), Sense::click());
            if resp.clicked() {
                self.tab = *tab;
            }
            let active = self.tab == *tab;
            let color = if active { theme::RED } else { theme::TAB_IDLE };
            // 图标 21px，标签 10.5px，整体上移一点给标签留位
            let icon = Rect::from_center_size(
                Pos2::new(cr.center().x, cr.top() + 21.0),
                Vec2::splat(21.0),
            );
            theme::tab_icon(&p, icon, i, color);
            p.text(
                Pos2::new(cr.center().x, cr.top() + 44.0),
                Align2::CENTER_CENTER,
                *label,
                theme::bold(10.5),
                color,
            );
        }
    }

    /* ============================ 主体分发 ============================ */

    pub fn body(&mut self, ui: &mut egui::Ui) {
        match self.tab {
            Tab::Predict => self.page_predict(ui),
            Tab::Check => self.page_check(ui),
            Tab::Analysis => self.page_analysis(ui),
            Tab::History => self.page_history(ui),
        }
    }

    pub fn set_tab(&mut self, t: Tab) {
        self.tab = t;
    }

    /* ======================= 页签一：推演 ======================= */

    fn page_predict(&mut self, ui: &mut egui::Ui) {
        // ---- 上期开奖 ----
        theme::card(ui, |ui| {
            let d = self.draws.last().cloned();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("上期开奖")
                            .size(12.0)
                            .color(theme::SUB),
                    );
                });
                ui.add_space(6.0);
                ui.label(theme::rb(
                    d.as_ref().map(|x| x.issue.clone()).unwrap_or_else(|| "—".into()),
                    14.5,
                    theme::INK,
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(d.as_ref().map(|x| x.date.clone()).unwrap_or_default())
                            .size(11.5)
                            .color(theme::SUB),
                    );
                });
            });
            ui.add_space(10.0);
            if let Some(d) = &d {
                theme::balls_row(ui, &d.f, &d.b, theme::BALL_D, None);
                ui.add_space(11.0);
                // 下一期预计
                let next = next_issue(&d.issue);
                let galley = ui.painter().layout_no_wrap(
                    "下一期预计 ".to_string(),
                    theme::font(12.0),
                    theme::SUB,
                );
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), galley.size().y + 4.0),
                    Sense::hover(),
                );
                let p = ui.painter();
                p.text(r.left_center(), Align2::LEFT_CENTER, "下一期预计 ", theme::font(12.0), theme::SUB);
                let g2 = p.layout_no_wrap(next, theme::bold(12.0), theme::RED);
                p.galley(
                    Pos2::new(r.left() + galley.size().x, r.center().y - g2.size().y * 0.5),
                    g2,
                    theme::RED,
                );
            }
        });

        // ---- 选号策略 ----
        theme::sec_title(ui, "选号策略");
        ui.horizontal_wrapped(|ui| {
            for s in Strategy::ALL {
                if theme::chip(ui, s.name(), self.strategy == s).clicked() {
                    self.strategy = s;
                }
            }
        });
        ui.add_space(8.0);
        theme::para(ui, self.strategy.desc(), 11.5, theme::SUB, 1.55);
        ui.add_space(10.0);

        // ---- 参数 ----
        theme::card(ui, |ui| {
            // 统计范围（CSS .row 上下 padding 9px）
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, 31.0), Sense::hover());
            let p = ui.painter();
            p.text(r.left_center(), Align2::LEFT_CENTER, "统计范围", theme::bold(13.0), theme::INK2);
            let txt = format!("近一年 · {} 期", self.window);
            p.text(r.right_center(), Align2::RIGHT_CENTER, txt, theme::font(12.5), theme::SUB);

            ui.add_space(9.0);
            theme::dashed_line(ui);
            ui.add_space(9.0);

            // 生成注数
            ui.horizontal(|ui| {
                ui.add_space(1.0);
                ui.label(theme::rb("生成注数", 13.0, theme::INK2));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(i) = theme::seg(ui, &["1 注", "3 注", "5 注", "10 注"], count_idx(self.count), false) {
                        self.count = [1usize, 3, 5, 10][i];
                    }
                });
            });
        });

        // ---- 开始推演 ----
        let resp = ui
            .vertical(|ui| {
                ui.add_space(2.0);
                theme::primary(ui, "开始推演", Some(theme::icon_bolt))
            })
            .inner;
        if resp.clicked() {
            self.run_predict();
        }
        ui.add_space(14.0);

        // ---- 结果 ----
        if let Some(res) = &self.predict {
            for s in &res.sets {
                pick_card(ui, s);
            }
            ui.add_space(2.0);
            let ms = self.predict_ms;
            let n = res.stats.n;
            theme::para(
                ui,
                &format!("本次推演用时 {:.1} ms · 参与统计 {} 期", ms, n),
                11.0,
                theme::FAINT,
                1.5,
            );
        }

        theme::disclaimer(
            ui,
            "彩票开奖为独立随机事件，任何统计与模型都无法提高中奖概率。本工具仅用于历史数据的统计分析，结果不构成投注建议，请理性购彩。",
        );
    }

    fn run_predict(&mut self) {
        let t = std::time::Instant::now();
        let opts = PredictOpts {
            strategy: self.strategy,
            window: self.window,
            count: self.count,
            seed: 0,
            filters: engine::Filters::default(),
        };
        let r = engine::predict(&self.draws, &opts);
        self.predict_ms = t.elapsed().as_secs_f64() * 1000.0;
        self.predict = Some(r);
    }

    /* ======================= 页签二：验票 ======================= */

    fn page_check(&mut self, ui: &mut egui::Ui) {
        let target = self.draws.get(self.ck_target).cloned();

        // ---- 核对期号 ----
        theme::card(ui, |ui| {
            let mut pick_open = false;
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("核对期号").size(11.0).color(theme::SUB));
                    ui.label(
                        theme::rb(
                            target.as_ref().map(|d| d.issue.clone()).unwrap_or_else(|| "—".into()),
                            16.0,
                            theme::INK,
                        ),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let r = ui.allocate_ui(Vec2::new(58.0, 28.0), |ui| {
                        theme::ghost(ui, "换期", 28.0, true)
                    });
                    pick_open = r.inner.clicked();
                });
            });
            ui.add_space(11.0);
            if let Some(d) = &target {
                theme::balls_row(ui, &d.f, &d.b, 29.0, None);
                ui.add_space(10.0);
                let note = format!(
                    "开奖日期 {} · 前区 5 个号 + 后区 2 个号",
                    d.date
                );
                theme::para(ui, &note, 11.5, theme::SUB, 1.5);
            }
            if pick_open {
                // 换期：往前一期
                if self.ck_target > 0 {
                    self.ck_target -= 1;
                    self.ck_outcome = None;
                }
            }
        });

        // ---- 单式 / 复式，不追加 / 追加（CSS .ck-seg：两个各占一半，间距 8）----
        let half = ((ui.available_width() - 9.0) * 0.5).max(80.0);
        ui.horizontal(|ui| {
            ui.add_space(1.0);
            if let Some(i) = theme::seg_fixed(
                ui,
                &["单式", "复式"],
                if self.ck_multi { 1 } else { 0 },
                half,
            ) {
                self.ck_multi = i == 1;
                if !self.ck_multi {
                    self.ck_front.truncate(FRONT_PICK);
                    self.ck_back.truncate(BACK_PICK);
                }
            }
            ui.add_space(8.0);
            if let Some(i) = theme::seg_fixed(
                ui,
                &["不追加", "追加"],
                if self.ck_extra { 1 } else { 0 },
                half,
            ) {
                self.ck_extra = i == 1;
            }
        });
        ui.add_space(12.0);

        // ---- 号码盘 ----
        theme::card(ui, |ui| {
            let fmax = if self.ck_multi { 35 } else { FRONT_PICK };
            let bmax = if self.ck_multi { 12 } else { BACK_PICK };

            pk_header(ui, "前区", self.ck_front.len(), FRONT_PICK);
            if pick_grid(ui, 1, FM as u8, 7, &mut self.ck_front, fmax, false) {
                self.ck_outcome = None;
            }
            ui.add_space(15.0);
            pk_header(ui, "后区", self.ck_back.len(), BACK_PICK);
            if pick_grid(ui, 1, BM as u8, 6, &mut self.ck_back, bmax, true) {
                self.ck_outcome = None;
            }
            ui.add_space(12.0);
            let foot = if self.ck_multi {
                format!("复式：前区选 {} 个、后区选 {} 个，按组合数计注", self.ck_front.len(), self.ck_back.len())
            } else {
                format!("单式：前区 {} / {}，后区 {} / {}", self.ck_front.len(), FRONT_PICK, self.ck_back.len(), BACK_PICK)
            };
            theme::para(ui, &foot, 11.5, theme::SUB, 1.5);
        });

        // ---- 操作 ----
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 8.0) * 0.5;
            ui.allocate_ui(Vec2::new(w, 40.0), |ui| {
                if theme::ghost(ui, "拍照对照", 40.0, false).clicked() {
                    // 相机与照片导入尚未接入
                }
            });
            ui.add_space(8.0);
            ui.allocate_ui(Vec2::new(w, 40.0), |ui| {
                if theme::ghost(ui, "清空", 40.0, false).clicked() {
                    self.ck_front.clear();
                    self.ck_back.clear();
                    self.ck_outcome = None;
                }
            });
        });
        ui.add_space(12.0);

        // ---- 核对 ----
        let resp = theme::primary(ui, "核对是否中奖", Some(theme::icon_check));
        if resp.clicked() {
            self.run_check();
        }
        ui.add_space(14.0);

        // ---- 结果 ----
        if let (Some(o), Some(t)) = (self.ck_outcome.clone(), target.clone()) {
            check_result(ui, &o, &t);
        }

        theme::sec_title(ui, "我的彩票");
        theme::card(ui, |ui| {
            theme::para(ui, "还没有保存的彩票", 12.5, theme::SUB, 1.5);
        });

        theme::disclaimer(
            ui,
            "验票结果按官方奖级规则本地计算，仅供核对参考；实际兑奖以体彩终端与官方公告为准。开奖号码来自内置历史数据。",
        );
    }

    fn run_check(&mut self) {
        if self.ck_front.len() < FRONT_PICK || self.ck_back.len() < BACK_PICK {
            self.say("号码不够：前区 5 个、后区 2 个");
            return;
        }
        let Some(t) = self.draws.get(self.ck_target).cloned() else {
            self.say("没有可核对的期次");
            return;
        };
        // 单式取前 5/2，复式取全部（判奖看最优组合）
        let o = ticket::check(&self.ck_front, &self.ck_back, &t);
        self.ck_outcome = Some(o);
    }

    /* ======================= 页签三：分析 ======================= */

    fn page_analysis(&mut self, ui: &mut egui::Ui) {
        let st = &self.stats;

        // ---- 前区热度 ----
        theme::card(ui, |ui| {
            card_head(ui, "前区热度（1–35）", &format!("近 {} 期", st.n));
            ui.add_space(12.0);
            heat_grid(ui, 1, FM as u8, 7, &st.freq_f, &st.omit_f, false);
            ui.add_space(10.0);
            // 图例
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(52.0, 7.0), Sense::hover());
                theme::gradient(
                    ui.painter(),
                    r,
                    Color32::from_rgb(0xF4, 0xF1, 0xF0),
                    theme::RED,
                    false,
                );
                ui.add_space(6.0);
                ui.label(egui::RichText::new("冷").size(10.5).color(theme::SUB));
                ui.label(egui::RichText::new("热").size(10.5).color(theme::SUB));
                ui.add_space(10.0);
                let (d, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
                ui.painter().circle_filled(d.center(), 4.5, Color32::from_rgb(0xD8, 0xDC, 0xE3));
                ui.add_space(4.0);
                ui.label(egui::RichText::new("角标为当前遗漏").size(10.5).color(theme::SUB));
            });
        });

        // ---- 后区热度 ----
        theme::card(ui, |ui| {
            card_head(ui, "后区热度（1–12）", "");
            ui.add_space(12.0);
            heat_grid(ui, 1, BM as u8, 6, &st.freq_b, &st.omit_b, true);
        });

        // ---- 和值分布 ----
        theme::card(ui, |ui| {
            card_head(
                ui,
                "前区和值分布",
                &format!(
                    "均值 {:.1} · 80% 区间 {:.0}–{:.0}",
                    st.sum_mean, st.sum_q10, st.sum_q90
                ),
            );
            ui.add_space(10.0);
            histogram(ui, &st.sums_sorted);
        });

        // ---- 奇偶结构 ----
        theme::card(ui, |ui| {
            card_head(ui, "奇偶结构", "");
            ui.add_space(12.0);
            let total: u32 = st.odd_dist.iter().sum();
            let max = *st.odd_dist.iter().max().unwrap_or(&1);
            for (i, &v) in st.odd_dist.iter().enumerate() {
                bar_row(
                    ui,
                    &format!("{} : {}", i, FRONT_PICK - i),
                    v,
                    max,
                    &format!("{:.0}%", pct(v, total)),
                    false,
                );
            }
        });

        // ---- 大小结构 ----
        theme::card(ui, |ui| {
            card_head(ui, "大小结构（1–17 / 18–35）", "");
            ui.add_space(12.0);
            let total: u32 = st.big_dist.iter().sum();
            let max = *st.big_dist.iter().max().unwrap_or(&1);
            for (i, &v) in st.big_dist.iter().enumerate() {
                bar_row(
                    ui,
                    &format!("{} 大", i),
                    v,
                    max,
                    &format!("{:.0}%", pct(v, total)),
                    true,
                );
            }
        });

        // ---- 五分区 ----
        theme::card(ui, |ui| {
            card_head(ui, "五分区分布", "");
            ui.add_space(12.0);
            let max = *st.zone_total.iter().max().unwrap_or(&1);
            let names = ["1–7", "8–14", "15–21", "22–28", "29–35"];
            let total: u32 = st.zone_total.iter().sum();
            for (i, &v) in st.zone_total.iter().enumerate() {
                bar_row(
                    ui,
                    names[i],
                    v,
                    max,
                    &format!("{:.0}%", pct(v, total)),
                    false,
                );
            }
        });

        // ---- 遗漏榜 ----
        theme::card(ui, |ui| {
            card_head(ui, "遗漏榜 · 前区", "距离上次出现");
            ui.add_space(12.0);
            let mut idx: Vec<(usize, usize)> = (1..=FM)
                .map(|n| (n, st.omit_f.get(n).copied().unwrap_or(0)))
                .collect();
            idx.sort_by(|a, b| b.1.cmp(&a.1));
            let top = (idx.len().min(8)).max(1);
            let max = idx.first().map(|x| x.1).unwrap_or(1).max(1);
            for (k, (n, om)) in idx.iter().take(top).enumerate() {
                omit_row(ui, *n as u8, *om, max, k > 0);
            }
        });

        theme::disclaimer(
            ui,
            "遗漏值只描述历史，不改变任何号码的下一期出现概率。理论上每个前区号码每期出现概率恒为 5/35 ≈ 14.29%。",
        );
    }

    /* ======================= 页签四：历史 ======================= */

    fn page_history(&mut self, ui: &mut egui::Ui) {
        // ---- 搜索框 ----
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 44.0), Sense::hover());
        ui.painter().rect_filled(rect, 13.0, theme::CARD);
        ui.painter().rect_stroke(
            rect,
            13.0,
            Stroke::new(1.0, theme::LINE),
            egui::StrokeKind::Inside,
        );
        let icon = Rect::from_center_size(
            Pos2::new(rect.left() + 22.0, rect.center().y),
            Vec2::splat(17.0),
        );
        let ip = ui.painter().clone();
        let k = icon.width() / 24.0;
        ip.circle_stroke(
            Pos2::new(icon.left() + 11.0 * k, icon.top() + 11.0 * k),
            6.5 * k,
            Stroke::new(2.0 * k, theme::SUB),
        );
        ip.line_segment(
            [
                Pos2::new(icon.left() + 16.0 * k, icon.top() + 16.0 * k),
                Pos2::new(icon.left() + 20.5 * k, icon.top() + 20.5 * k),
            ],
            Stroke::new(2.0 * k, theme::SUB),
        );

        let edit_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 40.0, rect.top() + 6.0),
            Pos2::new(rect.right() - 10.0, rect.bottom() - 6.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(edit_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(
                    egui::RichText::new("搜索期号 / 号码，如 26123 或 13 18 19")
                        .size(14.0)
                        .color(Color32::from_rgb(0xAE, 0xB4, 0xBF)),
                )
                .desired_width(f32::INFINITY)
                .frame(egui::Frame::NONE)
                .text_color(theme::INK)
                .font(egui::FontId::proportional(14.0)),
        );
        ui.add_space(10.0);

        // ---- 过滤 ----
        let q = self.query.trim().to_string();
        let filtered: Vec<&Draw> = self
            .draws
            .iter()
            .rev()
            .filter(|d| {
                if q.is_empty() {
                    return true;
                }
                d.issue.contains(&q) || d.line_text().replace(' ', "").contains(&q.replace(' ', ""))
            })
            .collect();
        let shown = filtered.len().min(self.hist_rows);

        // ---- 列表头 ----
        ui.horizontal(|ui| {
            ui.add_space(3.0);
            ui.label(
                egui::RichText::new(format!("共 {} 期", filtered.len()))
                    .size(11.5)
                    .color(theme::SUB),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(3.0);
                ui.label(
                    egui::RichText::new("点击查看期次特征")
                        .size(11.5)
                        .color(theme::SUB),
                );
            });
        });
        ui.add_space(8.0);

        // ---- 列表 ----
        for d in filtered.iter().take(shown) {
            hist_item(ui, d);
        }

        if shown < filtered.len() {
            ui.add_space(12.0);
            if theme::ghost(ui, "加载更多", 42.0, false).clicked() {
                self.hist_rows += 30;
            }
        }
        ui.add_space(8.0);
    }
}

/* ============================ 局部组件 ============================ */

fn short_date(d: &str) -> String {
    if d.len() >= 10 {
        d[5..10].to_string()
    } else {
        d.to_string()
    }
}

/// 期号 +1，保持 5 位
fn next_issue(issue: &str) -> String {
    issue
        .parse::<u32>()
        .map(|n| format!("{:05}", n + 1))
        .unwrap_or_else(|_| "—".into())
}

fn count_idx(c: usize) -> usize {
    match c {
        1 => 0,
        3 => 1,
        5 => 2,
        _ => 3,
    }
}

fn pct(v: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        v as f64 * 100.0 / total as f64
    }
}

/// 卡片标题行：左侧标题，右侧浅灰小字
fn card_head(ui: &mut egui::Ui, title: &str, right: &str) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    let p = ui.painter();
    p.text(r.left_center(), Align2::LEFT_CENTER, title, theme::bold(14.5), theme::INK);
    if !right.is_empty() {
        p.text(r.right_center(), Align2::RIGHT_CENTER, right, theme::font(11.5), theme::SUB);
    }
}

/// 推演结果里的一注
fn pick_card(ui: &mut egui::Ui, s: &engine::PickSet) {
    let w = ui.available_width();
    let start = ui.cursor().min;
    egui::Frame::new()
        .fill(theme::CARD)
        .stroke(Stroke::new(1.0, theme::LINE))
        .corner_radius(15.0)
        .inner_margin(egui::Margin::symmetric(13, 12))
        .shadow(egui::epaint::Shadow {
            offset: [0, 3],
            blur: 14,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(15, 19, 25, 24),
        })
        .show(ui, |ui| {
            ui.set_width(w - 26.0);
            ui.horizontal(|ui| {
                // 名次徽标
                let (rk, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                let bg = if s.rank == 1 { theme::RED } else { theme::SEG_BG };
                let fg = if s.rank == 1 { Color32::WHITE } else { theme::SUB };
                ui.painter().rect_filled(rk, 6.0, bg);
                ui.painter().text(
                    rk.center(),
                    Align2::CENTER_CENTER,
                    format!("{}", s.rank),
                    theme::bold(11.0),
                    fg,
                );
                ui.add_space(1.0);
                theme::balls_row(ui, &s.f, &s.b, 26.0, None);
            });
            ui.add_space(9.0);
            // 特征行 + 契合度
            ui.horizontal(|ui| {
                ui.add_space(1.0);
                ui.label(
                    egui::RichText::new(format!(
                        "和值 {} · 跨度 {} · 奇偶 {}:{} · 大小 {}:{}",
                        s.sum,
                        s.span,
                        s.odd_f,
                        FRONT_PICK as u8 - s.odd_f,
                        s.big_f,
                        FRONT_PICK as u8 - s.big_f
                    ))
                    .size(11.5)
                    .color(theme::SUB),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let txt = format!("{}", s.score.round() as i32);
                    let g = ui
                        .painter()
                        .layout_no_wrap(txt.clone(), theme::bold(12.0), theme::RED);
                    let (r, _) = ui.allocate_exact_size(
                        Vec2::new(g.size().x + 14.0, 20.0),
                        Sense::hover(),
                    );
                    ui.painter().rect_filled(r, 7.0, theme::RED_SOFT);
                    ui.painter()
                        .text(r.center(), Align2::CENTER_CENTER, txt, theme::bold(12.0), theme::RED);
                });
            });
            if !s.tags.is_empty() {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    for t in &s.tags {
                        let g = ui.painter().layout_no_wrap(
                            t.to_string(),
                            theme::bold(10.5),
                            theme::INK2,
                        );
                        let (r, _) = ui.allocate_exact_size(
                            Vec2::new(g.size().x + 14.0, g.size().y + 4.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(r, 6.0, theme::CHIP_BG);
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            *t,
                            theme::bold(10.5),
                            theme::INK2,
                        );
                    }
                });
            }
        });
    let end = ui.cursor().min;
    let _ = (start, end);
    ui.add_space(9.0);
}

/// 号码盘小标题：前区/后区 + 已选计数
fn pk_header(ui: &mut egui::Ui, name: &str, sel: usize, need: usize) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    let p = ui.painter();
    p.text(r.left_center(), Align2::LEFT_CENTER, name, theme::bold(13.0), theme::INK2);
    p.text(
        r.right_center(),
        Align2::RIGHT_CENTER,
        format!("已选 {} / {}", sel, need),
        theme::font(11.5),
        theme::SUB,
    );
    ui.add_space(9.0);
}

/// 号码网格。返回 true 表示选择发生变化。
fn pick_grid(
    ui: &mut egui::Ui,
    from: u8,
    to: u8,
    cols: usize,
    sel: &mut Vec<u8>,
    max: usize,
    blue: bool,
) -> bool {
    let n = (to - from + 1) as usize;
    let rows = (n + cols - 1) / cols;
    let gap = 5.0;
    let w = ui.available_width();
    let cell = (w - gap * (cols - 1) as f32) / cols as f32;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(w, cell * rows as f32 + gap * (rows as f32 - 1.0)),
        Sense::hover(),
    );
    let p = ui.painter().clone();
    let mut changed = false;

    for i in 0..n {
        let num = from + i as u8;
        let r = i / cols;
        let c = i % cols;
        let cr = Rect::from_min_size(
            Pos2::new(rect.left() + (cell + gap) * c as f32, rect.top() + (cell + gap) * r as f32),
            Vec2::splat(cell),
        );
        let on = sel.contains(&num);
        let resp = ui.interact(cr, ui.id().with(("pk", blue, num)), Sense::click());
        if resp.clicked() {
            changed = true;
            if on {
                sel.retain(|x| *x != num);
            } else if sel.len() < max {
                sel.push(num);
                sel.sort_unstable();
            }
        }
        if on {
            theme::draw_ball(&p, cr.center(), cell, num, !blue, false);
        } else {
            p.circle_filled(cr.center(), cell * 0.5, theme::CHIP_BG);
            p.text(
                cr.center(),
                Align2::CENTER_CENTER,
                format!("{:02}", num),
                theme::bold(13.5),
                theme::INK2,
            );
        }
    }
    changed
}

/// 验票结果卡
fn check_result(ui: &mut egui::Ui, o: &ticket::Outcome, t: &Draw) {
    let win = o.is_win();
    let w = ui.available_width();
    egui::Frame::new()
        .fill(if win {
            Color32::from_rgb(0xFF, 0xEE, 0xEA)
        } else {
            theme::CARD
        })
        .stroke(Stroke::new(
            1.0,
            if win {
                Color32::from_rgb(0xFB, 0xD9, 0xD3)
            } else {
                theme::LINE
            },
        ))
        .corner_radius(theme::CARD_R)
        .inner_margin(egui::Margin::symmetric(16, 16))
        .show(ui, |ui| {
            ui.set_width(w - 32.0);
            // 标签行
            let (r, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), 18.0),
                Sense::hover(),
            );
            ui.painter().text(
                r.left_center(),
                Align2::LEFT_CENTER,
                if win { "恭喜中奖" } else { "未中奖" },
                theme::bold(12.5),
                if win { theme::RED } else { theme::SUB },
            );
            ui.add_space(6.0);
            // 大奖级
            let (r2, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), if win { 38.0 } else { 30.0 }),
                Sense::hover(),
            );
            ui.painter().text(
                r2.left_center(),
                Align2::LEFT_CENTER,
                o.name(),
                theme::bold(if win { 30.0 } else { 22.0 }),
                if win { theme::RED } else { theme::INK },
            );
            ui.add_space(2.0);
            ui.label(theme::rb(o.hit_text(), 12.0, theme::SUB));

            // 命中分布
            ui.add_space(12.0);
            hit_row(ui, "开奖", &t.f, &t.b, None, None);
            ui.add_space(8.0);
            hit_row(
                ui,
                "命中",
                &t.f,
                &t.b,
                Some(&o.front_mask),
                Some(&o.back_mask),
            );

            if let Some(amt) = o.amount() {
                ui.add_space(14.0);
                theme::dashed_line(ui);
                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("参考奖金")
                            .size(12.5)
                            .color(theme::INK2),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(theme::rb(format!("¥{}", amt), 17.0, theme::RED));
                    });
                });
            }
        });
    ui.add_space(14.0);
}

/// 一行「开奖 / 命中」球
fn hit_row(
    ui: &mut egui::Ui,
    label: &str,
    f: &[u8],
    b: &[u8],
    fm: Option<&[bool; 5]>,
    bm: Option<&[bool; 2]>,
) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(30.0, 26.0), Sense::hover());
        ui.painter().text(
            r.left_center(),
            Align2::LEFT_CENTER,
            label,
            theme::font(11.0),
            theme::SUB,
        );
        let mut all: Vec<bool> = Vec::new();
        if let Some(m) = fm {
            all.extend_from_slice(m);
        }
        if let Some(m) = bm {
            all.extend_from_slice(m);
        }
        let mask = if all.is_empty() { None } else { Some(all.as_slice()) };
        theme::balls_row(ui, f, b, 26.0, mask);
    });
}

/// 分析页热度格
fn heat_grid(
    ui: &mut egui::Ui,
    from: u8,
    to: u8,
    cols: usize,
    freq: &[u32],
    omit: &[usize],
    blue: bool,
) {
    let n = (to - from + 1) as usize;
    let rows = (n + cols - 1) / cols;
    let gap = 5.0;
    let w = ui.available_width();
    let cell = (w - gap * (cols - 1) as f32) / cols as f32;
    let h = cell * 0.92;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(w, h * rows as f32 + gap * (rows as f32 - 1.0)),
        Sense::hover(),
    );
    let p = ui.painter().clone();

    // 归一化频率
    let vals: Vec<f64> = (from..=to)
        .map(|i| freq.get(i as usize).copied().unwrap_or(0) as f64)
        .collect();
    let max = vals.iter().cloned().fold(1.0, f64::max);
    let min = vals.iter().cloned().fold(f64::MAX, f64::min);

    for i in 0..n {
        let num = from + i as u8;
        let r = i / cols;
        let c = i % cols;
        let cr = Rect::from_min_size(
            Pos2::new(rect.left() + (cell + gap) * c as f32, rect.top() + (h + gap) * r as f32),
            Vec2::new(cell, h),
        );
        let t = if max > min {
            ((vals[i] - min) / (max - min)) as f32
        } else {
            0.5
        };
        // 严格照原版：hsl(4,72%,96-t*52%) / hsl(222,78%,95-t*48%)
        let bg = if blue {
            theme::hsl(222.0, 0.78, (95.0 - t * 48.0) / 100.0)
        } else {
            theme::hsl(4.0, 0.72, (96.0 - t * 52.0) / 100.0)
        };
        p.rect_filled(cr, 9.0, bg);
        // 深色底用白字，浅色底用深字（原版阈值 t > 0.70）
        let light = t > 0.70;
        let fg = if light {
            Color32::WHITE
        } else if blue {
            Color32::from_rgb(0x28, 0x40, 0x6F)
        } else {
            Color32::from_rgb(0x3A, 0x41, 0x50)
        };
        p.text(
            Pos2::new(cr.center().x, cr.center().y + 1.0),
            Align2::CENTER_CENTER,
            format!("{:02}", num),
            theme::bold(13.0),
            fg,
        );
        // 右上角遗漏角标：8px，透明度随底色反转
        let om = omit.get(num as usize).copied().unwrap_or(0);
        if om > 0 {
            let ic = if light {
                Color32::from_rgba_unmultiplied(255, 255, 255, 210)
            } else if blue {
                Color32::from_rgba_unmultiplied(40, 64, 111, 107)
            } else {
                Color32::from_rgba_unmultiplied(58, 65, 80, 107)
            };
            p.text(
                Pos2::new(cr.right() - 3.0, cr.top() + 1.0),
                Align2::RIGHT_TOP,
                format!("{}", om),
                theme::bold(8.5),
                ic,
            );
        }
    }
}

/// 一条横向柱：标签 + 槽 + 数值
fn bar_row(ui: &mut egui::Ui, label: &str, v: u32, max: u32, value: &str, blue: bool) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    let p = ui.painter();
    let cy = r.center().y;
    p.text(
        Pos2::new(r.left(), cy),
        Align2::LEFT_CENTER,
        label,
        theme::bold(12.0),
        theme::INK2,
    );
    let bar_l = r.left() + 62.0;
    let bar_r = r.right() - 48.0;
    if bar_r > bar_l {
        let bar = Rect::from_min_max(Pos2::new(bar_l, cy - 4.5), Pos2::new(bar_r, cy + 4.5));
        let ratio = if max == 0 { 0.0 } else { v as f32 / max as f32 };
        let (c0, c1) = if blue {
            (Color32::from_rgb(0x8A, 0xB0, 0xFF), theme::BLUE)
        } else {
            (Color32::from_rgb(0xF2, 0x7C, 0x6E), theme::RED)
        };
        theme::hbar(p, bar, ratio, c0, c1);
    }
    p.text(
        Pos2::new(r.right(), cy),
        Align2::RIGHT_CENTER,
        value,
        theme::font(11.5),
        theme::SUB,
    );
    ui.add_space(8.0);
}

/// 遗漏榜一行
fn omit_row(ui: &mut egui::Ui, num: u8, omit: usize, max: usize, divider: bool) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::hover());
    let p = ui.painter();
    let cy = r.center().y;
    if divider {
        p.line_segment(
            [Pos2::new(r.left(), r.top()), Pos2::new(r.right(), r.top())],
            Stroke::new(1.0, theme::CHIP_BG),
        );
    }
    p.text(
        Pos2::new(r.left(), cy),
        Align2::LEFT_CENTER,
        format!("{:02}", num),
        theme::bold(13.0),
        theme::INK,
    );
    p.text(
        Pos2::new(r.left() + 26.0, cy),
        Align2::LEFT_CENTER,
        "遗漏",
        theme::font(12.0),
        theme::SUB,
    );
    let txt = format!("{} 期", omit);
    let g = p.layout_no_wrap(txt.clone(), theme::bold(13.5), theme::RED);
    p.galley(
        Pos2::new(r.left() + 56.0, cy - g.size().y * 0.5),
        g,
        theme::RED,
    );
    // 右侧比例条
    let bar_r = r.right();
    let bar_l = bar_r - 86.0;
    let bar = Rect::from_min_max(Pos2::new(bar_l, cy - 3.0), Pos2::new(bar_r, cy + 3.0));
    let ratio = if max == 0 { 0.0 } else { omit as f32 / max as f32 };
    theme::hbar(
        p,
        bar,
        ratio,
        Color32::from_rgb(0xFF, 0xB4, 0xAC),
        theme::RED,
    );
}

/// 和值直方图（纯手绘，不依赖 canvas）
fn histogram(ui: &mut egui::Ui, sums: &[f64]) {
    let w = ui.available_width();
    let h = 158.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let p = ui.painter().clone();
    if sums.is_empty() {
        return;
    }
    // 照原版 drawHistogram(..., 40, 160, ...)：40 个桶
    let lo = sums.iter().cloned().fold(f64::MAX, f64::min).floor();
    let hi = sums.iter().cloned().fold(f64::MIN, f64::max).ceil();
    let bins = 40usize;
    let span = (hi - lo).max(1.0);
    let mut counts = vec![0u32; bins];
    for &s in sums {
        let k = (((s - lo) / span) * bins as f64).floor() as usize;
        counts[k.min(bins - 1)] += 1;
    }
    let max = counts.iter().copied().max().unwrap_or(1).max(1);

    let base = rect.bottom() - 16.0;
    let plot_top = rect.top() + 6.0;
    // 三条横向网格线，让高度可读
    for k in 1..=3 {
        let y = base - (base - plot_top) * k as f32 / 3.0;
        let mut x = rect.left();
        while x < rect.right() {
            let x2 = (x + 3.0).min(rect.right());
            p.line_segment(
                [Pos2::new(x, y), Pos2::new(x2, y)],
                Stroke::new(1.0, theme::LINE.gamma_multiply(0.7)),
            );
            x += 6.0;
        }
    }

    let bw = rect.width() / bins as f32;
    for (i, &c) in counts.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let bh = (c as f32 / max as f32) * (base - plot_top);
        let x = rect.left() + bw * i as f32 + 0.5;
        let r = Rect::from_min_max(
            Pos2::new(x, base - bh),
            Pos2::new(x + (bw - 1.0).max(1.0), base),
        );
        let t = i as f32 / (bins - 1) as f32;
        theme::gradient(
            &p,
            r,
            Color32::from_rgb(0xF2, 0x7C, 0x6E).lerp_to_gamma(theme::RED, t),
            Color32::from_rgb(0xE0, 0x45, 0x3C).lerp_to_gamma(Color32::from_rgb(0xAE, 0x1C, 0x16), t),
            true,
        );
    }
    // 底轴
    p.line_segment(
        [
            Pos2::new(rect.left(), base),
            Pos2::new(rect.right(), base),
        ],
        Stroke::new(1.0, theme::LINE),
    );
    p.text(
        Pos2::new(rect.left(), rect.bottom() - 1.0),
        Align2::LEFT_BOTTOM,
        format!("{}", lo as i32),
        theme::font(10.5),
        theme::SUB,
    );
    p.text(
        Pos2::new(rect.right(), rect.bottom() - 1.0),
        Align2::RIGHT_BOTTOM,
        format!("{}", hi as i32),
        theme::font(10.5),
        theme::SUB,
    );
}

/// 历史列表一项
fn hist_item(ui: &mut egui::Ui, d: &Draw) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 52.0), Sense::click());
    let p = ui.painter().clone();
    p.rect_filled(
        rect,
        13.0,
        if resp.is_pointer_button_down_on() {
            Color32::from_rgb(0xFA, 0xFB, 0xFC)
        } else {
            theme::CARD
        },
    );
    p.rect_stroke(
        rect,
        13.0,
        Stroke::new(1.0, theme::LINE),
        egui::StrokeKind::Inside,
    );

    let left = rect.left() + 12.0;
    p.text(
        Pos2::new(left, rect.center().y - 8.0),
        Align2::LEFT_CENTER,
        &d.issue,
        theme::bold(13.0),
        theme::INK,
    );
    p.text(
        Pos2::new(left, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        short_date(&d.date),
        theme::font(10.5),
        theme::SUB,
    );

    // 号码球：第一颗起点固定在 66 + 内边距处
    let balls_left = left + 74.0;
    let dsz = 23.0;
    let gap = 4.0;
    let mut x = balls_left + dsz * 0.5;
    let cy = rect.center().y;
    for &n in d.f.iter() {
        theme::draw_ball(&p, Pos2::new(x, cy), dsz, n, true, false);
        x += dsz + gap;
    }
    x += 3.0;
    p.line_segment(
        [Pos2::new(x, cy - 7.0), Pos2::new(x, cy + 7.0)],
        Stroke::new(1.0, theme::LINE),
    );
    x += 4.0 + dsz * 0.5;
    for &n in d.b.iter() {
        theme::draw_ball(&p, Pos2::new(x, cy), dsz, n, false, false);
        x += dsz + gap;
    }
    ui.add_space(7.0);
}

/* ============================ 主循环 ============================ */

#[cfg(feature = "native")]
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

impl App {
    /// 绘制整个界面。与 eframe 解耦，离屏渲染器（src/bin/render.rs）也走这里，
    /// 保证「截图检查」和真机跑的是同一份代码。
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let bg = ui.max_rect();
        ui.painter().rect_filled(bg, 0.0, theme::BG);

        ui.vertical(|ui| {
            self.header(ui);
            self.sync_bar(ui);

            let scroll_h = (ui.available_height() - theme::TABBAR_H).max(60.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: 14,
                            right: 14,
                            top: 12,
                            bottom: 20,
                        })
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            self.body(ui);
                        });
                });
            self.tabbar(ui);
        });

        // toast
        if let Some((msg, left)) = &mut self.toast {
            let dt = ui.ctx().input(|i| i.stable_dt).min(0.1) as f64;
            *left -= dt;
            let text = msg.clone();
            theme::toast(ui, &text);
            if *left <= 0.0 {
                self.toast = None;
            } else {
                ui.ctx().request_repaint();
            }
        }
    }
}
