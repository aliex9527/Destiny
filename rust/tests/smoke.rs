//! 无头冒烟测试：不开窗口也能把界面跑一帧，用来抓布局 panic 与索引越界。
//!
//! 桌面预览窗口在本机起不来（环境没有可用 OpenGL），所以界面正确性靠这里兜。
//! 运行：cargo test

use destiny::{app::{App, Tab}, data, engine, ticket};

#[test]
fn 四个页签都能渲染不崩溃() {
    let mut app = App::new(None);
    assert!(!app.draws.is_empty(), "内嵌数据没解析出来");

    // 逐页渲染。egui 的测试入口会跑完整的布局，越界/除零会直接 panic。
    for tab in [Tab::Predict, Tab::Check, Tab::Analysis, Tab::History] {
        app.tab = tab;
        egui::__run_test_ui(|ui| {
            app.body(ui);
        });
    }
}

#[test]
fn 生成号码满足规则() {
    let draws = data::baseline();
    let opts = engine::PredictOpts {
        strategy: engine::Strategy::Balanced,
        window: 100,
        count: 5,
        seed: 12345,
        ..Default::default()
    };
    let res = engine::predict(&draws, &opts);
    assert_eq!(res.sets.len(), 5, "应生成 5 注");

    let mut seen_hist = std::collections::HashSet::new();
    for d in &draws {
        seen_hist.insert(d.f);
    }
    for s in &res.sets {
        // 前区 5 个互不相同且在 1..=35
        let mut f = s.f;
        f.sort_unstable();
        assert_eq!(f.len(), 5);
        assert!(f.windows(2).all(|w| w[0] < w[1]), "前区必须递增且不重复");
        assert!(f.iter().all(|&v| (1..=35).contains(&v)));
        assert!(s.b.iter().all(|&v| (1..=12).contains(&v)));
        assert!(s.b[0] < s.b[1], "后区必须递增");
        assert!(!seen_hist.contains(&s.f), "不应与历史开奖重复：{:?}", s.f);
        assert!(s.score >= 0.0 && s.score <= 1.0, "评分应在 0~1：{}", s.score);
    }
}

#[test]
fn 同一随机种子结果可复现() {
    let draws = data::baseline();
    let mk = || engine::PredictOpts {
        strategy: engine::Strategy::Hot,
        window: 50,
        count: 3,
        seed: 777,
        ..Default::default()
    };
    let a = engine::predict(&draws, &mk());
    let b = engine::predict(&draws, &mk());
    let fa: Vec<_> = a.sets.iter().map(|s| s.f).collect();
    let fb: Vec<_> = b.sets.iter().map(|s| s.f).collect();
    assert_eq!(fa, fb, "固定种子下两次生成应完全一致");
}

#[test]
fn 奖级判定覆盖九等() {
    // 官方奖级：5+2 一等 … 0+2 九等
    let cases = [
        (5, 2, 1),
        (5, 1, 2),
        (5, 0, 3),
        (4, 2, 4),
        (4, 1, 5),
        (3, 2, 6),
        (4, 0, 7),
        (3, 1, 8),
        (2, 2, 8),
        (3, 0, 9),
        (2, 1, 9),
        (1, 2, 9),
        (0, 2, 9),
        (2, 0, 0),
        (0, 0, 0),
    ];
    for (f, b, want) in cases {
        assert_eq!(ticket::level_of(f, b), want, "{} + {} 应为 {} 等奖", f, b, want);
    }
}

#[test]
fn 数据解析拒绝脏数据() {
    assert!(data::parse_line("26123,13,18,19,20,28,9,11,2026-10-28").is_some());
    // 字段数不对
    assert!(data::parse_line("26123,13,18,19,20,28,9").is_none());
    // 前区越界
    assert!(data::parse_line("26123,13,18,19,20,36,9,11,2026-10-28").is_none());
    // 前区没有递增
    assert!(data::parse_line("26123,13,18,19,20,20,9,11,2026-10-28").is_none());
    // 后区越界
    assert!(data::parse_line("26123,13,18,19,20,28,9,13,2026-10-28").is_none());
}
