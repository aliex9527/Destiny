//! 投注核对
//!
//! 大乐透奖级共九等：前区 5 码、后区 2 码全部命中的组合决定奖级，
//! 一等奖与二等奖为浮动奖金，三至九等奖固定。

use crate::data::Draw;

pub const LEVEL_NAMES: [&str; 10] = [
    "未中奖", "一等奖", "二等奖", "三等奖", "四等奖", "五等奖", "六等奖", "七等奖", "八等奖", "九等奖",
];

/// 固定奖金；一、二等奖为浮动，返回 None
pub fn fixed_amount(level: usize) -> Option<u64> {
    match level {
        3 => Some(10000),
        4 => Some(3000),
        5 => Some(300),
        6 => Some(200),
        7 => Some(100),
        8 => Some(15),
        9 => Some(5),
        _ => None,
    }
}

/// 命中 (前区, 后区) 对应的奖级，0 表示未中奖
pub fn level_of(front_hits: usize, back_hits: usize) -> usize {
    match (front_hits, back_hits) {
        (5, 2) => 1,
        (5, 1) => 2,
        (5, 0) => 3,
        (4, 2) => 4,
        (4, 1) => 5,
        (3, 2) => 6,
        (4, 0) => 7,
        (3, 1) | (2, 2) => 8,
        (3, 0) | (2, 1) | (1, 2) | (0, 2) => 9,
        _ => 0,
    }
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub front_hits: usize,
    pub back_hits: usize,
    pub level: usize,
    pub front_mask: [bool; 5],
    pub back_mask: [bool; 2],
}

impl Outcome {
    pub fn name(&self) -> &'static str {
        LEVEL_NAMES[self.level.min(9)]
    }

    pub fn is_win(&self) -> bool {
        self.level > 0
    }

    /// 固定奖金；浮动奖返回 None
    pub fn amount(&self) -> Option<u64> {
        fixed_amount(self.level)
    }

    /// 单注命中描述，如「中 3+1」
    pub fn hit_text(&self) -> String {
        format!("{} + {}", self.front_hits, self.back_hits)
    }
}

pub fn check(f: &[u8], b: &[u8], target: &Draw) -> Outcome {
    let mut front_mask = [false; 5];
    let mut back_mask = [false; 2];
    let mut front_hits = 0;
    let mut back_hits = 0;
    for (i, &v) in f.iter().enumerate().take(5) {
        if target.f.contains(&v) {
            front_hits += 1;
            front_mask[i] = true;
        }
    }
    for (i, &v) in b.iter().enumerate().take(2) {
        if target.b.contains(&v) {
            back_hits += 1;
            back_mask[i] = true;
        }
    }
    Outcome {
        front_hits,
        back_hits,
        level: level_of(front_hits, back_hits),
        front_mask,
        back_mask,
    }
}

/// 核对多注，返回每一注的结果
pub fn check_many(tickets: &[([u8; 5], [u8; 2])], target: &Draw) -> Vec<Outcome> {
    tickets.iter().map(|(f, b)| check(f, b, target)).collect()
}

/// 从一行文本解析号码，支持「1 2 3 4 5 + 6 7」「0102030405-0607」等写法
pub fn parse_numbers(text: &str) -> Option<([u8; 5], [u8; 2])> {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_ascii_digit() { c } else { ' ' })
        .collect();
    let nums: Vec<u8> = cleaned
        .split_whitespace()
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();
    let nums: Vec<u8> = if nums.len() == 7 {
        nums
    } else {
        // 允许连写：0102030405 0607 → 两位一组
        let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() == 14 {
            (0..7)
                .filter_map(|i| digits[i * 2..i * 2 + 2].parse::<u8>().ok())
                .collect()
        } else {
            return None;
        }
    };
    if nums.len() != 7 {
        return None;
    }
    let mut f = [0u8; 5];
    f.copy_from_slice(&nums[0..5]);
    let mut b = [0u8; 2];
    b.copy_from_slice(&nums[5..7]);
    f.sort_unstable();
    b.sort_unstable();
    if f.iter().any(|&v| v < 1 || v > 35) || b.iter().any(|&v| v < 1 || v > 12) {
        return None;
    }
    if f.windows(2).any(|w| w[0] == w[1]) || b[0] == b[1] {
        return None;
    }
    Some((f, b))
}
