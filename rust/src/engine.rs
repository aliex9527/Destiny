//! 大乐透分析引擎
//!
//! 移植自 web 版的 `engine.js`，保持算法逐行对应，便于两种实现互相对照。
//! 前区 1..=35 取 5，后区 1..=12 取 2。
//!
//! 约定：所有以号码为下标的表长度都是 `MAX+1`，索引 0 弃用，
//! 这样 `table[n]` 直接就是号码 n 的值，不用处处 -1。

use crate::data::{Draw, BACK_PICK, BM, FM, FRONT_PICK};

/// 单个号码的理论平均间隔：前区 35/5 = 7 期
pub const EXP_GAP_F: f64 = FM as f64 / FRONT_PICK as f64;
/// 后区 12/2 = 6 期
pub const EXP_GAP_B: f64 = BM as f64 / BACK_PICK as f64;

/* ============================ 基础工具 ============================ */

/// mulberry32：与 web 版同一套伪随机，seed 相同则结果完全一致。
pub struct Rng {
    a: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self { a: seed }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.a = self.a.wrapping_add(0x6D2B_79F5);
        let mut t = (self.a ^ (self.a >> 15)).wrapping_mul(1 | self.a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }
}

pub fn gauss(x: f64, mu: f64, sig: f64) -> f64 {
    let sig = if sig == 0.0 { 1.0 } else { sig };
    let d = (x - mu) / sig;
    (-0.5 * d * d).exp()
}

pub fn mean(a: &[f64]) -> f64 {
    if a.is_empty() {
        0.0
    } else {
        a.iter().sum::<f64>() / a.len() as f64
    }
}

/// 样本标准差（除以 n-1），与 web 版一致
pub fn std_dev(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 2 {
        return 0.0;
    }
    let m = mean(a);
    let s: f64 = a.iter().map(|x| (x - m) * (x - m)).sum();
    (s / (n - 1) as f64).sqrt()
}

/// 线性插值分位数，输入必须已升序
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = (sorted.len() - 1) as f64 * q;
    let base = pos.floor() as usize;
    let rest = pos - base as f64;
    if base + 1 < sorted.len() {
        sorted[base] + rest * (sorted[base + 1] - sorted[base])
    } else {
        sorted[base]
    }
}

/* ============================ 单期特征 ============================ */

#[derive(Clone, Debug, Default)]
pub struct Features {
    pub sum_f: u16,
    pub span: u8,
    pub odd_f: u8,
    pub big_f: u8,
    pub zones: [u8; 5],
    pub tails: [u8; 10],
    pub max_run: u8,
    pub consec_pairs: u8,
    pub ac: i32,
    pub sum_b: u8,
    pub odd_b: u8,
    pub big_b: u8,
}

pub fn draw_features(f: &[u8], b: &[u8]) -> Features {
    let mut fs = f.to_vec();
    fs.sort_unstable();
    let mut bs = b.to_vec();
    bs.sort_unstable();

    let mut ft = Features::default();
    ft.sum_f = fs.iter().map(|&x| x as u16).sum();
    if let (Some(&first), Some(&last)) = (fs.first(), fs.last()) {
        ft.span = last - first;
    }

    for &v in &fs {
        if v % 2 == 1 {
            ft.odd_f += 1;
        }
        if v >= 18 {
            ft.big_f += 1;
        }
        // 每 7 个号一区：1-7 / 8-14 / 15-21 / 22-28 / 29-35
        let z = ((v as usize - 1) / 7).min(4);
        ft.zones[z] += 1;
        ft.tails[(v % 10) as usize] += 1;
    }

    // 最长连号串与连号对数
    let mut run = 1u8;
    let mut max_run = 1u8;
    for w in fs.windows(2) {
        if w[1] == w[0] + 1 {
            run += 1;
            ft.consec_pairs += 1;
            if run > max_run {
                max_run = run;
            }
        } else {
            run = 1;
        }
    }
    ft.max_run = max_run;

    // AC 值 = 差值种类数 - (号码数 - 1)
    let mut seen = [false; FM + 1];
    let mut kinds = 0i32;
    for i in 0..fs.len() {
        for j in (i + 1)..fs.len() {
            let d = (fs[j] - fs[i]) as usize;
            if d <= FM && !seen[d] {
                seen[d] = true;
                kinds += 1;
            }
        }
    }
    ft.ac = kinds - (fs.len() as i32 - 1);

    ft.sum_b = bs.iter().map(|&x| x as u16).sum::<u16>() as u8;
    for &v in &bs {
        if v % 2 == 1 {
            ft.odd_b += 1;
        }
        if v >= 7 {
            ft.big_b += 1;
        }
    }
    ft
}

/* ============================ 统计汇总 ============================ */

pub struct Stats {
    pub n: usize,
    pub total: usize,
    pub freq_f: Vec<u32>,
    pub freq_b: Vec<u32>,
    pub omit_f: Vec<usize>,
    pub omit_b: Vec<usize>,
    pub max_omit_f: Vec<usize>,
    pub max_omit_b: Vec<usize>,
    pub avg_gap_f: Vec<f64>,
    pub feats: Vec<Features>,
    pub sum_mean: f64,
    pub sum_std: f64,
    pub sum_q10: f64,
    pub sum_q25: f64,
    pub sum_q75: f64,
    pub sum_q90: f64,
    pub sum_min: f64,
    pub sum_max: f64,
    pub span_mean: f64,
    pub span_std: f64,
    pub span_q10: f64,
    pub span_q90: f64,
    pub odd_dist: [u32; 6],
    pub big_dist: [u32; 6],
    pub zone_total: [u32; 5],
    pub tail_total: [u32; 10],
    pub consec_dist: [u32; 5],
    pub back_sum_dist: [u32; 25],
    pub sums_sorted: Vec<f64>,
}

impl Stats {
    pub fn empty() -> Self {
        Stats {
            n: 0,
            total: 0,
            freq_f: vec![0; FM + 1],
            freq_b: vec![0; BM + 1],
            omit_f: vec![0; FM + 1],
            omit_b: vec![0; BM + 1],
            max_omit_f: vec![0; FM + 1],
            max_omit_b: vec![0; BM + 1],
            avg_gap_f: vec![0.0; FM + 1],
            feats: Vec::new(),
            sum_mean: 0.0,
            sum_std: 0.0,
            sum_q10: 0.0,
            sum_q25: 0.0,
            sum_q75: 0.0,
            sum_q90: 0.0,
            sum_min: 0.0,
            sum_max: 0.0,
            span_mean: 0.0,
            span_std: 0.0,
            span_q10: 0.0,
            span_q90: 0.0,
            odd_dist: [0; 6],
            big_dist: [0; 6],
            zone_total: [0; 5],
            tail_total: [0; 10],
            consec_dist: [0; 5],
            back_sum_dist: [0; 25],
            sums_sorted: Vec::new(),
        }
    }
}

/// win = 0 或大于总数时按全量统计
pub fn build_stats(draws: &[Draw], win: usize) -> Stats {
    let total = draws.len();
    let n = if win == 0 || win >= total { total } else { win };
    let seg = &draws[total.saturating_sub(n)..];

    let mut st = Stats::empty();
    st.n = n;
    st.total = total;
    if n == 0 {
        return st;
    }

    st.omit_f = vec![n; FM + 1];
    st.omit_b = vec![n; BM + 1];

    let mut last_f = vec![-1i64; FM + 1];

    st.feats.reserve(n);
    for d in seg {
        for &v in &d.f {
            st.freq_f[v as usize] += 1;
        }
        for &v in &d.b {
            st.freq_b[v as usize] += 1;
        }
        st.feats.push(draw_features(&d.f, &d.b));
    }

    // 当前遗漏：从最近一期往回扫，第一次遇到即为其遗漏期数
    for (i, d) in seg.iter().enumerate().rev() {
        let dist = n - 1 - i;
        for &v in &d.f {
            let vi = v as usize;
            if last_f[vi] == -1 {
                last_f[vi] = i as i64;
                st.omit_f[vi] = dist;
            }
        }
        for &v in &d.b {
            let vi = v as usize;
            if st.omit_b[vi] == n {
                st.omit_b[vi] = dist;
            }
        }
    }

    // 历史最大遗漏
    for v in 1..=FM {
        let mut best = 0usize;
        let mut last: i64 = -1;
        for (i, d) in seg.iter().enumerate() {
            if d.f.contains(&(v as u8)) {
                if last >= 0 {
                    best = best.max(i - last as usize - 1);
                }
                last = i as i64;
            }
        }
        if last >= 0 {
            best = best.max(n - 1 - last as usize);
        }
        st.max_omit_f[v] = if best != 0 {
            best
        } else if st.omit_f[v] > 0 {
            st.omit_f[v]
        } else {
            0
        };
    }
    for v in 1..=BM {
        let mut best = 0usize;
        let mut last: i64 = -1;
        for (i, d) in seg.iter().enumerate() {
            if d.b.contains(&(v as u8)) {
                if last >= 0 {
                    best = best.max(i - last as usize - 1);
                }
                last = i as i64;
            }
        }
        if last >= 0 {
            best = best.max(n - 1 - last as usize);
        }
        st.max_omit_b[v] = best;
    }

    let mut sums: Vec<f64> = st.feats.iter().map(|f| f.sum_f as f64).collect();
    let mut spans: Vec<f64> = st.feats.iter().map(|f| f.span as f64).collect();
    sums.sort_by(|a, b| a.partial_cmp(b).unwrap());
    spans.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for f in &st.feats {
        let oi = (f.odd_f as usize).min(5);
        st.odd_dist[oi] += 1;
        let bi = (f.big_f as usize).min(5);
        st.big_dist[bi] += 1;
        for k in 0..5 {
            st.zone_total[k] += f.zones[k] as u32;
        }
        for k in 0..10 {
            st.tail_total[k] += f.tails[k] as u32;
        }
        let ci = (f.consec_pairs as usize).min(4);
        st.consec_dist[ci] += 1;
        let sb = f.sum_b as usize;
        if sb < st.back_sum_dist.len() {
            st.back_sum_dist[sb] += 1;
        }
    }

    for v in 1..=FM {
        let c = st.freq_f[v] as f64;
        st.avg_gap_f[v] = if c > 0.0 { n as f64 / c } else { n as f64 };
    }

    st.sum_mean = mean(&sums);
    st.sum_std = std_dev(&sums);
    st.sum_q10 = quantile(&sums, 0.10);
    st.sum_q25 = quantile(&sums, 0.25);
    st.sum_q75 = quantile(&sums, 0.75);
    st.sum_q90 = quantile(&sums, 0.90);
    st.sum_min = *sums.first().unwrap_or(&0.0);
    st.sum_max = *sums.last().unwrap_or(&0.0);
    st.span_mean = mean(&spans);
    st.span_std = std_dev(&spans);
    st.span_q10 = quantile(&spans, 0.10);
    st.span_q90 = quantile(&spans, 0.90);
    st.sums_sorted = sums;
    st
}

/* ============================ 共现矩阵 ============================ */

pub struct CoMatrix {
    /// m[a][b] = a 与 b 同期出现的次数（对称）
    pub m: Vec<Vec<u32>>,
    /// cnt[a] = a 出现的期数
    pub cnt: Vec<u32>,
}

pub fn co_matrix(draws: &[Draw], win: usize) -> CoMatrix {
    let total = draws.len();
    let n = if win == 0 || win >= total { total } else { win };
    let seg = &draws[total.saturating_sub(n)..];
    let mut m = vec![vec![0u32; FM + 1]; FM + 1];
    let mut cnt = vec![0u32; FM + 1];
    for d in seg {
        for i in 0..d.f.len() {
            let a = d.f[i] as usize;
            if a > FM {
                continue;
            }
            cnt[a] += 1;
            for j in (i + 1)..d.f.len() {
                let b = d.f[j] as usize;
                if b > FM {
                    continue;
                }
                m[a][b] += 1;
                m[b][a] += 1;
            }
        }
    }
    CoMatrix { m, cnt }
}

/* ============================ 策略 ============================ */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Strategy {
    Balanced,
    Hot,
    Cold,
    Omit,
    Co,
    Random,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OmitMode {
    Gauss,
    Inc,
    Regress,
}

pub struct Weights {
    pub freq: f64,
    pub omit: f64,
    pub co: f64,
    pub tail: f64,
    pub zone: f64,
}

impl Strategy {
    pub const ALL: [Strategy; 6] = [
        Strategy::Balanced,
        Strategy::Hot,
        Strategy::Cold,
        Strategy::Omit,
        Strategy::Co,
        Strategy::Random,
    ];

    pub fn key(&self) -> &'static str {
        match self {
            Strategy::Balanced => "balanced",
            Strategy::Hot => "hot",
            Strategy::Cold => "cold",
            Strategy::Omit => "omit",
            Strategy::Co => "co",
            Strategy::Random => "random",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Strategy::Balanced => "均衡型",
            Strategy::Hot => "热号追踪",
            Strategy::Cold => "冷号回补",
            Strategy::Omit => "遗漏回归",
            Strategy::Co => "关联传递",
            Strategy::Random => "纯随机",
        }
    }

    pub fn desc(&self) -> &'static str {
        match self {
            Strategy::Balanced => "频率、遗漏、关联综合加权，特征最贴近历史分布",
            Strategy::Hot => "偏重近期高频号，追热不追冷",
            Strategy::Cold => "偏重长期未出与超长遗漏号",
            Strategy::Omit => "遗漏接近历史均值倍数的号码优先",
            Strategy::Co => "偏重与上期号码历史共现率高的号",
            Strategy::Random => "等概率抽样，作为对照组基准",
        }
    }

    fn weights(&self) -> Weights {
        match self {
            Strategy::Balanced => Weights { freq: 0.30, omit: 0.25, co: 0.20, tail: 0.10, zone: 0.15 },
            Strategy::Hot => Weights { freq: 0.55, omit: 0.08, co: 0.22, tail: 0.05, zone: 0.10 },
            Strategy::Cold => Weights { freq: 0.12, omit: 0.50, co: 0.13, tail: 0.10, zone: 0.15 },
            Strategy::Omit => Weights { freq: 0.15, omit: 0.50, co: 0.10, tail: 0.10, zone: 0.15 },
            Strategy::Co => Weights { freq: 0.15, omit: 0.15, co: 0.50, tail: 0.10, zone: 0.10 },
            Strategy::Random => Weights { freq: 0.0, omit: 0.0, co: 0.0, tail: 0.0, zone: 0.0 },
        }
    }

    fn omit_mode(&self) -> OmitMode {
        match self {
            Strategy::Cold => OmitMode::Inc,
            Strategy::Omit => OmitMode::Regress,
            _ => OmitMode::Gauss,
        }
    }
}

/* ============================ 权重 ============================ */

fn front_weights(stats: &Stats, matrix: &CoMatrix, strat: Strategy, last: Option<&Draw>) -> Vec<f64> {
    let w = strat.weights();
    let mode = strat.omit_mode();
    let mut out = vec![0.0f64; FM + 1];

    let max_freq = (1..=FM).map(|i| stats.freq_f[i]).max().unwrap_or(1).max(1) as f64;
    let last_front: &[u8] = last.map(|d| &d.f[..]).unwrap_or(&[]);

    for v in 1..=FM {
        let freq_term = stats.freq_f[v] as f64 / max_freq;
        let r = stats.omit_f[v] as f64 / EXP_GAP_F;
        let omit_term = match mode {
            OmitMode::Inc => 1.0 - (-r / 1.6).exp(),
            OmitMode::Regress => gauss(r, 1.6, 0.8),
            OmitMode::Gauss => gauss(r, 1.0, 1.1),
        };

        // 与上期号码的平均历史共现率
        let mut co_term = 0.0;
        if !last_front.is_empty() {
            let mut acc = 0.0;
            let mut cntc = 0;
            for &o in last_front {
                let oi = o as usize;
                if oi == v || oi > FM {
                    continue;
                }
                let denom = matrix.cnt[v].min(matrix.cnt[oi]).max(1) as f64;
                acc += matrix.m[v][oi] as f64 / denom;
                cntc += 1;
            }
            co_term = if cntc > 0 { acc / cntc as f64 } else { 0.0 };
        }

        let tail = v % 10;
        let tail_base = stats.n as f64 * FRONT_PICK as f64 / 10.0 + 1e-9;
        let mut tail_term = 1.0 - (stats.tail_total[tail] as f64 / tail_base);
        tail_term = (tail_term * 0.5 + 0.5).clamp(0.0, 1.0);

        let z = ((v - 1) / 7).min(4);
        let zone_base = stats.n as f64 * FRONT_PICK as f64 / 5.0 + 1e-9;
        let mut zone_term = 1.0 - (stats.zone_total[z] as f64 / zone_base);
        zone_term = (zone_term * 0.5 + 0.5).clamp(0.0, 1.0);

        let raw = w.freq * freq_term
            + w.omit * omit_term
            + w.co * co_term
            + w.tail * tail_term
            + w.zone * zone_term;
        // 基础权重保证任何号码都有机会
        out[v] = 0.06 + raw;
        if strat == Strategy::Random {
            out[v] = 1.0;
        }
    }
    out
}

fn back_weights(stats: &Stats, strat: Strategy) -> Vec<f64> {
    let w = strat.weights();
    let mode = strat.omit_mode();
    let mut out = vec![0.0f64; BM + 1];
    let max_freq = (1..=BM).map(|i| stats.freq_b[i]).max().unwrap_or(1).max(1) as f64;
    for v in 1..=BM {
        if strat == Strategy::Random {
            out[v] = 1.0;
            continue;
        }
        let freq_term = stats.freq_b[v] as f64 / max_freq;
        let r = stats.omit_b[v] as f64 / EXP_GAP_B;
        let omit_term = match mode {
            OmitMode::Inc => 1.0 - (-r / 1.6).exp(),
            OmitMode::Regress => gauss(r, 1.6, 0.9),
            OmitMode::Gauss => gauss(r, 1.0, 1.2),
        };
        let wf = if w.freq == 0.0 { 0.3 } else { w.freq };
        let wo = if w.omit == 0.0 { 0.25 } else { w.omit };
        out[v] = 0.08 + wf * freq_term + wo * omit_term;
    }
    out
}

/// 按权重不放回抽样，返回升序号码
fn sample_weighted(weights: &[f64], k: usize, max_idx: usize, rng: &mut Rng) -> Vec<u8> {
    let mut pool: Vec<usize> = (1..=max_idx).collect();
    let mut wts: Vec<f64> = (1..=max_idx).map(|i| weights[i]).collect();
    let mut out = Vec::with_capacity(k);
    for _ in 0..k {
        if pool.is_empty() {
            break;
        }
        let tot: f64 = wts.iter().sum();
        if tot <= 0.0 {
            out.push(pool.remove(0) as u8);
            wts.remove(0);
            continue;
        }
        let r = rng.next_f64() * tot;
        let mut acc = 0.0;
        let mut idx = pool.len() - 1;
        for i in 0..pool.len() {
            acc += wts[i];
            if r <= acc {
                idx = i;
                break;
            }
        }
        out.push(pool.remove(idx) as u8);
        wts.remove(idx);
    }
    out.sort_unstable();
    out
}

/* ============================ 贴合度 ============================ */

pub fn shape_score(f: &[u8], b: &[u8], stats: &Stats) -> (f64, Features) {
    let ft = draw_features(f, b);
    let s_sum = gauss(ft.sum_f as f64, stats.sum_mean, stats.sum_std.max(6.0));
    let s_span = gauss(ft.span as f64, stats.span_mean, stats.span_std.max(3.0));
    let odd_rate = stats.odd_dist[(ft.odd_f as usize).min(5)] as f64 / stats.n.max(1) as f64;
    let big_rate = stats.big_dist[(ft.big_f as usize).min(5)] as f64 / stats.n.max(1) as f64;
    let zmax = *ft.zones.iter().max().unwrap_or(&0);
    let s_zone = if zmax >= 4 {
        0.25
    } else if zmax == 3 {
        0.8
    } else {
        1.0
    };
    let s_run = if ft.max_run >= 4 {
        0.3
    } else if ft.max_run == 3 {
        0.85
    } else {
        1.0
    };
    let score = 0.34 * s_sum
        + 0.20 * s_span
        + 0.22 * (odd_rate * 3.0).min(1.0)
        + 0.14 * (big_rate * 3.0).min(1.0)
        + 0.05 * s_zone
        + 0.05 * s_run;
    (score, ft)
}

/* ============================ 选号 ============================ */

#[derive(Clone, Debug)]
pub struct PickSet {
    pub f: [u8; 5],
    pub b: [u8; 2],
    pub score: f64,
    pub shape: f64,
    pub sum: u16,
    pub span: u8,
    pub odd_f: u8,
    pub big_f: u8,
    pub consec: u8,
    pub ac: i32,
    pub tags: Vec<&'static str>,
    pub rank: usize,
}

pub struct Filters {
    pub sum_lo: Option<u16>,
    pub sum_hi: Option<u16>,
    pub odd_lo: u8,
    pub odd_hi: u8,
    pub no_hist: bool,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            sum_lo: None,
            sum_hi: None,
            odd_lo: 1,
            odd_hi: 4,
            no_hist: true,
        }
    }
}

pub struct PredictOpts {
    pub strategy: Strategy,
    pub window: usize,
    pub count: usize,
    pub seed: u32,
    pub filters: Filters,
}

impl Default for PredictOpts {
    fn default() -> Self {
        PredictOpts {
            strategy: Strategy::Balanced,
            window: 100,
            count: 5,
            seed: 0,
            filters: Filters::default(),
        }
    }
}

pub struct PredictResult {
    pub sets: Vec<PickSet>,
    pub stats: Stats,
    pub matrix: CoMatrix,
    pub strategy: Strategy,
    pub window: usize,
    pub seed: u32,
    pub meta_sum_lo: f64,
    pub meta_sum_hi: f64,
    pub meta_odd_lo: u8,
    pub meta_odd_hi: u8,
}

pub fn predict(draws: &[Draw], opts: &PredictOpts) -> PredictResult {
    let seed = if opts.seed == 0 {
        // 与 JS 的 Date.now() & 0x7fffffff 等价：调用方不指定时也要有变化
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0))
            & 0x7fff_ffff
    } else {
        opts.seed
    };
    let mut rng = Rng::new(seed);
    let stats = build_stats(draws, opts.window);
    let matrix = co_matrix(draws, opts.window);
    let last = draws.last();
    let wf = front_weights(&stats, &matrix, opts.strategy, last);
    let wb = back_weights(&stats, opts.strategy);

    let sum_lo = opts.filters.sum_lo.map(|v| v as f64).unwrap_or(stats.sum_q10);
    let sum_hi = opts.filters.sum_hi.map(|v| v as f64).unwrap_or(stats.sum_q90);
    let odd_lo = opts.filters.odd_lo;
    let odd_hi = opts.filters.odd_hi;
    let no_hist = opts.filters.no_hist;

    let mut hist: std::collections::HashSet<[u8; 5]> = std::collections::HashSet::new();
    if no_hist {
        for d in draws {
            hist.insert(d.f);
        }
    }

    let wf_max = wf.iter().cloned().fold(0.0f64, f64::max);
    let wb_max = wb.iter().cloned().fold(0.0f64, f64::max);

    let mut seen: std::collections::HashSet<[u8; 5]> = std::collections::HashSet::new();
    let mut selected: Vec<PickSet> = Vec::new();
    let max_guard = opts.count * 400 + 2000;
    let mut guard = 0;

    while selected.len() < opts.count && guard < max_guard {
        guard += 1;
        let fv = sample_weighted(&wf, FRONT_PICK, FM, &mut rng);
        let bv = sample_weighted(&wb, BACK_PICK, BM, &mut rng);
        if fv.len() != FRONT_PICK || bv.len() != BACK_PICK {
            continue;
        }
        let mut f = [0u8; 5];
        f.copy_from_slice(&fv);
        let mut b = [0u8; 2];
        b.copy_from_slice(&bv);

        let ft = draw_features(&f, &b);
        if (ft.sum_f as f64) < sum_lo || (ft.sum_f as f64) > sum_hi {
            continue;
        }
        if ft.odd_f < odd_lo || ft.odd_f > odd_hi {
            continue;
        }
        if *ft.zones.iter().max().unwrap_or(&0) >= 5 {
            continue;
        }
        if ft.max_run >= 5 {
            continue;
        }
        if seen.contains(&f) {
            continue;
        }
        if no_hist && hist.contains(&f) {
            continue;
        }
        seen.insert(f);

        let (sscore, sft) = shape_score(&f, &b, &stats);
        let ind_raw: f64 = f.iter().map(|&v| wf[v as usize]).sum::<f64>() / FRONT_PICK as f64;
        let ind = ind_raw / (1.0 + wf_max);
        let indb_raw: f64 = b.iter().map(|&v| wb[v as usize]).sum::<f64>() / BACK_PICK as f64;
        let indb = indb_raw / (1.0 + wb_max);
        let total = 0.62 * sscore + 0.26 * (ind * 2.0).min(1.0) + 0.12 * (indb * 2.0).min(1.0);

        selected.push(PickSet {
            f,
            b,
            score: total,
            shape: sscore,
            sum: sft.sum_f,
            span: sft.span,
            odd_f: sft.odd_f,
            big_f: sft.big_f,
            consec: sft.consec_pairs,
            ac: sft.ac,
            tags: Vec::new(),
            rank: 0,
        });
    }

    selected.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    for (i, s) in selected.iter_mut().enumerate() {
        let mut tags = Vec::new();
        tags.push(if s.odd_f >= 3 { "偏奇" } else { "偏偶" });
        tags.push(if s.big_f >= 3 { "大号多" } else { "小号多" });
        if s.consec > 0 {
            tags.push("含连号");
        }
        if (s.sum as f64) > stats.sum_q75 {
            tags.push("和值偏高");
        } else if (s.sum as f64) < stats.sum_q25 {
            tags.push("和值偏低");
        }
        s.tags = tags;
        s.rank = i + 1;
    }

    PredictResult {
        sets: selected,
        stats,
        matrix,
        strategy: opts.strategy,
        window: opts.window,
        seed,
        meta_sum_lo: sum_lo,
        meta_sum_hi: sum_hi,
        meta_odd_lo: odd_lo,
        meta_odd_hi: odd_hi,
    }
}

/* ============================ 组合工具 ============================ */

pub fn combos(arr: &[u8], k: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur = Vec::with_capacity(k);
    fn rec(arr: &[u8], k: usize, start: usize, cur: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..arr.len() {
            cur.push(arr[i]);
            rec(arr, k, i + 1, cur, out);
            cur.pop();
        }
    }
    rec(arr, k, 0, &mut cur, &mut out);
    out
}

fn uniq_sorted(a: &[u8]) -> Vec<u8> {
    let mut v = a.to_vec();
    v.sort_unstable();
    v.dedup();
    v
}

/// 胆拖：胆码固定，拖码补足
pub fn dan_tuo(dan: &[u8], tuo: &[u8], back_dan: &[u8], back_tuo: &[u8]) -> Vec<([u8; 5], [u8; 2])> {
    if dan.len() > FRONT_PICK {
        return Vec::new();
    }
    let need = FRONT_PICK - dan.len();
    let mut ft = tuo.to_vec();
    ft.sort_unstable();
    let fc = combos(&ft, need);
    let bn = BACK_PICK.saturating_sub(back_dan.len());
    let mut bt = back_tuo.to_vec();
    bt.sort_unstable();
    let bc: Vec<Vec<u8>> = if bn == 0 { vec![Vec::new()] } else { combos(&bt, bn) };

    let mut out = Vec::new();
    for c in &fc {
        let mut fv: Vec<u8> = dan.to_vec();
        fv.extend_from_slice(c);
        let fv = uniq_sorted(&fv);
        if fv.len() != FRONT_PICK {
            continue;
        }
        for d in &bc {
            let mut bv: Vec<u8> = back_dan.to_vec();
            bv.extend_from_slice(d);
            let bv = uniq_sorted(&bv);
            if bv.len() != BACK_PICK {
                continue;
            }
            let mut f = [0u8; 5];
            f.copy_from_slice(&fv);
            let mut b = [0u8; 2];
            b.copy_from_slice(&bv);
            out.push((f, b));
        }
    }
    out
}

pub struct CompoundResult {
    pub list: Vec<([u8; 5], [u8; 2])>,
    pub kept: usize,
    pub origin: usize,
}

/// 复式展开，可选按和值/奇偶缩水
pub fn compound(front: &[u8], back: &[u8], shrink: bool, sum_lo: u16, sum_hi: u16, odd_lo: u8, odd_hi: u8) -> CompoundResult {
    let mut ff = front.to_vec();
    ff.sort_unstable();
    let mut bb = back.to_vec();
    bb.sort_unstable();
    if ff.len() < FRONT_PICK || bb.len() < BACK_PICK {
        return CompoundResult { list: Vec::new(), kept: 0, origin: 0 };
    }
    let fc = combos(&ff, FRONT_PICK);
    let bc = combos(&bb, BACK_PICK);
    let mut all = Vec::with_capacity(fc.len() * bc.len());
    for c in &fc {
        for d in &bc {
            let mut f = [0u8; 5];
            f.copy_from_slice(c);
            let mut b = [0u8; 2];
            b.copy_from_slice(d);
            all.push((f, b));
        }
    }
    let origin = all.len();
    if !shrink {
        return CompoundResult { list: all, kept: origin, origin };
    }
    let kept: Vec<([u8; 5], [u8; 2])> = all
        .into_iter()
        .filter(|(f, _)| {
            let s: u16 = f.iter().map(|&x| x as u16).sum();
            let odd = f.iter().filter(|&&x| x % 2 == 1).count() as u8;
            s >= sum_lo && s <= sum_hi && odd >= odd_lo && odd <= odd_hi
        })
        .collect();
    let k = kept.len();
    CompoundResult { list: kept, kept: k, origin }
}

/* ============================ 回测 ============================ */

pub struct BacktestRow {
    pub issue: String,
    pub date: String,
    pub real: [u8; 5],
    pub back: [u8; 2],
    pub best_f: usize,
    pub best_b: usize,
}

pub struct BacktestResult {
    pub tested: usize,
    pub strategy: Strategy,
    pub m: usize,
    pub window: usize,
    pub rows: Vec<BacktestRow>,
    pub hit_f: [usize; 6],
    pub hit_b: [usize; 3],
    pub best_front: usize,
    pub best_back: usize,
    pub prize: usize,
    pub avg_f: f64,
    pub rand_avg_f: f64,
    pub rand_best: usize,
}

pub fn backtest(draws: &[Draw], n_periods: usize, m: usize, window: usize, strat: Strategy) -> BacktestResult {
    let start = (window + 1).max(draws.len().saturating_sub(n_periods));
    let mut rows = Vec::new();
    let mut hit_f = [0usize; 6];
    let mut hit_b = [0usize; 3];
    let mut best_front = 0;
    let mut best_back = 0;
    let mut prize = 0;
    let mut random_hit_f = 0usize;
    let mut random_best = 0usize;
    let mut sum_best_f = 0usize;

    for i in start..draws.len() {
        let hist = &draws[..i];
        let real = &draws[i];
        let opts = PredictOpts {
            strategy: strat,
            window,
            count: m,
            seed: 10000 + i as u32,
            filters: Filters { no_hist: true, ..Default::default() },
        };
        let res = predict(hist, &opts);
        let mut bf = 0usize;
        let mut bb = 0usize;
        for s in &res.sets {
            let cf = s.f.iter().filter(|v| real.f.contains(v)).count();
            let cb = s.b.iter().filter(|v| real.b.contains(v)).count();
            bf = bf.max(cf);
            bb = bb.max(cb);
        }
        hit_f[bf.min(5)] += 1;
        hit_b[bb.min(2)] += 1;
        best_front = best_front.max(bf);
        best_back = best_back.max(bb);
        if bf >= 3 && bb >= 2 {
            prize += 1;
        }
        sum_best_f += bf;

        // 对照组：纯随机 m 注
        let mut rng = Rng::new(20000 + i as u32);
        let ones = vec![1.0f64; FM + 1];
        let mut rb = 0usize;
        for _ in 0..m {
            let rf = sample_weighted(&ones, FRONT_PICK, FM, &mut rng);
            let cf = rf.iter().filter(|v| real.f.contains(v)).count();
            rb = rb.max(cf);
        }
        random_hit_f += rb;
        random_best = random_best.max(rb);

        rows.push(BacktestRow {
            issue: real.issue.clone(),
            date: real.date.clone(),
            real: real.f,
            back: real.b,
            best_f: bf,
            best_b: bb,
        });
    }

    let tested = rows.len();
    BacktestResult {
        tested,
        strategy: strat,
        m,
        window,
        hit_f,
        hit_b,
        best_front,
        best_back,
        prize,
        avg_f: if tested > 0 { sum_best_f as f64 / tested as f64 } else { 0.0 },
        rand_avg_f: if tested > 0 { random_hit_f as f64 / tested as f64 } else { 0.0 },
        rand_best: random_best,
        rows,
    }
}

/* ============================ 号码画像 ============================ */

pub struct NumberProfile {
    pub num: u8,
    pub total: usize,
    pub rate: f64,
    pub omit: usize,
    pub max_omit: usize,
    pub avg_gap: f64,
    pub threshold: f64,
    pub freq: u32,
    pub recent: Vec<String>,
}

pub fn number_profile(draws: &[Draw], stats: &Stats, num: u8) -> NumberProfile {
    let mut hits = 0usize;
    let mut recent = Vec::new();
    for d in draws {
        if d.f.contains(&num) {
            hits += 1;
            recent.push(d.issue.clone());
        }
    }
    let tail: Vec<String> = recent.iter().rev().take(5).cloned().collect();
    let n = num as usize;
    let avg_gap = stats.avg_gap_f.get(n).copied().unwrap_or(0.0);
    NumberProfile {
        num,
        total: hits,
        rate: if draws.is_empty() { 0.0 } else { hits as f64 / draws.len() as f64 },
        omit: stats.omit_f.get(n).copied().unwrap_or(0),
        max_omit: stats.max_omit_f.get(n).copied().unwrap_or(0),
        avg_gap,
        threshold: avg_gap * 1.5,
        freq: stats.freq_f.get(n).copied().unwrap_or(0),
        recent: tail,
    }
}
