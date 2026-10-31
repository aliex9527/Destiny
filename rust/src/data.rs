//! 开奖数据模型
//!
//! 内嵌一份基线数据（`assets/draws.txt`，由 `tools/gen_rust_data.py` 从 web 版
//! 的 data.js 生成），运行时若本地存储里有更新过的数据则优先用本地的。

use std::path::{Path, PathBuf};

pub const FM: usize = 35;
pub const BM: usize = 12;
pub const FRONT_PICK: usize = 5;
pub const BACK_PICK: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draw {
    pub issue: String,
    pub date: String,
    pub f: [u8; 5],
    pub b: [u8; 2],
}

impl Draw {
    /// "26123" 这样的期号，用于显示
    pub fn issue_short(&self) -> &str {
        &self.issue
    }

    /// 形如「13 18 19 20 28 + 09 11」的一行文本
    pub fn line_text(&self) -> String {
        let f: Vec<String> = self.f.iter().map(|v| format!("{:02}", v)).collect();
        let b: Vec<String> = self.b.iter().map(|v| format!("{:02}", v)).collect();
        format!("{} + {}", f.join(" "), b.join(" "))
    }
}

const RAW: &str = include_str!("../assets/draws.txt");

/// 解析一行：期号,前5,后2,日期
pub fn parse_line(line: &str) -> Option<Draw> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() != 9 {
        return None;
    }
    let issue = parts[0].trim().to_string();
    if issue.is_empty() {
        return None;
    }
    let mut f = [0u8; 5];
    for i in 0..5 {
        let v: u8 = parts[1 + i].trim().parse().ok()?;
        if v < 1 || v as usize > FM {
            return None;
        }
        f[i] = v;
    }
    let mut b = [0u8; 2];
    for i in 0..2 {
        let v: u8 = parts[6 + i].trim().parse().ok()?;
        if v < 1 || v as usize > BM {
            return None;
        }
        b[i] = v;
    }
    // 前区必须严格递增且不重复，脏数据直接丢掉
    for i in 1..5 {
        if f[i] <= f[i - 1] {
            return None;
        }
    }
    for i in 1..2 {
        if b[i] <= b[i - 1] {
            return None;
        }
    }
    let date = parts[8].trim().to_string();
    Some(Draw { issue, f, b, date })
}

pub fn parse_raw(raw: &str) -> Vec<Draw> {
    raw.lines().filter_map(parse_line).collect()
}

/// 内嵌的基线数据（打包时冻结在 .so 里）
pub fn baseline() -> Vec<Draw> {
    parse_raw(RAW)
}

/* ============================ 持久化 ============================ */

/// 数据文件放在应用私有目录，不需要任何存储权限。
pub fn store_path(base: &Path) -> PathBuf {
    base.join("draws.txt")
}

pub fn save(path: &Path, draws: &[Draw]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut buf = String::with_capacity(draws.len() * 36);
    for d in draws {
        buf.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            d.issue, d.f[0], d.f[1], d.f[2], d.f[3], d.f[4], d.b[0], d.b[1], d.date
        ));
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(buf.as_bytes())?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

pub fn load(path: &Path) -> Option<Vec<Draw>> {
    let text = std::fs::read_to_string(path).ok()?;
    let v = parse_raw(&text);
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
