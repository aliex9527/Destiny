//! 验证推演结果「契合度」徽章的显示值（app.rs: (score*100).round()）。
//! 原版 JS 是 (score*100).toFixed(0)，v2.2.3 及之前 Rust 版漏乘 100，
//! 四舍五入后只剩 0 或 1——这正是用户看到的「大小 2:3 后面那个 1」。
//!
//! 跑法：cargo test --release --test verify_badge -- --nocapture

use destiny::{data, engine};

#[test]
fn badge_shows_percent() {
    let draws = data::baseline();
    let opts = engine::PredictOpts {
        strategy: engine::Strategy::Balanced,
        window: 100,
        count: 5,
        seed: 0,
        filters: engine::Filters::default(),
    };
    let r = engine::predict(&draws, &opts);
    println!("--- 契合度徽章显示值（score*100 取整）---");
    for (i, s) in r.sets.iter().enumerate() {
        let shown_old = s.score.round() as i32; // 修复前：0 或 1
        let shown_new = (s.score * 100.0).round() as i32; // 修复后
        println!(
            "第{}注  score={:.3}  旧显示={}  新显示={}  特征: 和值 {} · 跨度 {} · 奇偶 {}:{} · 大小 {}:{}",
            i + 1,
            s.score,
            shown_old,
            shown_new,
            s.sum,
            s.span,
            s.odd_f,
            5 - s.odd_f,
            s.big_f,
            5 - s.big_f,
        );
        assert!(
            (40..=100).contains(&shown_new),
            "契合度应在 40~100 之间，实际 {}",
            shown_new
        );
    }
}
