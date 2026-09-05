use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;

use calamine::{Data, Range, Reader, Xlsx};
use regex::Regex;

use crate::model::ColData;

/// 允许读取的表格扩展名
pub const ALLOWED_EXTS: [&str; 4] = ["xlsx", "xlsm", "xltx", "xltm"];

/// Windows 文件名不允许出现的字符
pub fn invalid_char_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"[|><?*":\\/]"#).unwrap())
}

/// 单元格内容转字符串（对齐 openpyxl + Python str() 的行为）
fn data_to_string(data: &Data) -> String {
    match data {
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                // 学号、序号这类整数，避免输出成 2021001.0
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Data::String(s) => s.trim().to_string(),
        Data::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Data::Empty => String::new(),
        other => other.to_string(),
    }
}

/// 读取工作簿中第一张工作表
pub fn load_range(path: &str) -> Result<Range<Data>, String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件：{}", e))?;
    let mut workbook: Xlsx<_> =
        Xlsx::new(BufReader::new(file)).map_err(|e| format!("无法读取表格，文件可能已损坏：{}", e))?;
    let mut sheets = workbook.worksheets();
    let (_, range) = sheets
        .drain(..)
        .next()
        .ok_or_else(|| "表格中没有任何工作表".to_string())?;
    Ok(range)
}

/// 一张工作表（行列序号沿用 openpyxl，从 1 开始）
pub struct SheetTable {
    range: Range<Data>,
    /// 表头所在行
    pub first_line: usize,
    /// 数据最后一行
    pub last_line: usize,
    /// 数据第一列
    pub first_col: usize,
    /// 数据最后一列
    pub last_col: usize,
}

impl SheetTable {
    pub fn new(range: Range<Data>) -> Self {
        Self {
            range,
            first_line: 1,
            last_line: 0,
            first_col: 1,
            last_col: 0,
        }
    }

    /// 取单元格内容，空单元格返回 None（row / col 从 1 开始）
    fn value(&self, row: usize, col: usize) -> Option<String> {
        if row == 0 || col == 0 {
            return None;
        }
        match self.range.get((row - 1, col - 1)) {
            Some(Data::Empty) | None => None,
            Some(data) => {
                let text = data_to_string(data);
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
        }
    }

    /// 判断表格格式是否正确
    ///
    /// 返回值：
    /// - `1`：表格正确
    /// - `0`：表格为空
    /// - `-1`：表格没超过 2 行
    /// - `2`：关键字有重复
    pub fn is_correct(&mut self) -> i32 {
        let (rows, cols) = self.range.get_size();
        if rows == 0 || cols == 0 {
            return 0;
        }
        if self.value(1, 1).is_none() {
            return 0;
        }
        if cols <= 1 {
            // 单列表格
            self.first_line = 1;
            self.find_last_line();
            if self.last_line < 2 {
                return -1;
            }
            self.first_col = 1;
            self.last_col = 1;
            return 1;
        }
        // 多列表格：在前 9 行里寻找表头
        for i in 1..10 {
            if self.value(i, 2).is_some() {
                self.first_line = i;
                self.find_last_line();
                if self.last_line < 2 {
                    return -1;
                }
                self.first_col = 1;
                self.find_last_col();
                let mut keys: Vec<String> = Vec::new();
                for c in self.first_col..=self.last_col {
                    keys.push(self.value(self.first_line, c).unwrap_or_default());
                }
                let uniq: HashSet<&String> = keys.iter().collect();
                if uniq.len() < keys.len() {
                    return 2;
                }
                return 1;
            }
        }
        0
    }

    fn find_last_line(&mut self) {
        let mut i = self.first_line;
        while self.value(i, 1).is_some() {
            i += 1;
        }
        self.last_line = i - 1;
    }

    fn find_last_col(&mut self) {
        let mut i = 1;
        while self.value(self.first_line, i).is_some() {
            i += 1;
        }
        self.last_col = i - 1;
    }

    /// 返回一列的数据并判断是否适合做关键字
    fn col_data(&self, col: usize) -> ColData {
        let key = self.value(self.first_line, col).unwrap_or_default();
        let mut values: Vec<String> = Vec::new();
        for row in (self.first_line + 1)..=self.last_line {
            values.push(self.value(row, col).unwrap_or_default());
        }

        let mut col = ColData {
            key,
            values,
            is_key_word: true,
            display: true,
            reason: "该项适合做关键字".to_string(),
            delta: 0,
        };

        // 判断表格值是否包含重复项
        let uniq: HashSet<&String> = col.values.iter().collect();
        col.delta = col.values.len() - uniq.len();
        if col.delta > 2 {
            col.is_key_word = false;
            col.reason = format!("该项不适合做关键字，因为至少有{}个重复项", col.delta);
        }

        // 判断表格值是否为序号
        let line_count = (self.last_line - self.first_line + 1) as f64;
        let mut sum = 0f64;
        let mut numeric = true;
        for v in &col.values {
            match v.trim().parse::<f64>() {
                Ok(f) => sum += f,
                Err(_) => {
                    numeric = false;
                    break;
                }
            }
        }
        if numeric && !col.values.is_empty() && sum < line_count * line_count {
            col.is_key_word = false;
            col.reason = "该项可能是序号，不适合做关键字".to_string();
        }

        // 判断这一项是否包含非法字符
        for v in &col.values {
            if invalid_char_re().is_match(v) {
                col.is_key_word = false;
                col.display = false;
                col.reason = "该项包含不符合文件名规定的字符|><?*\":\\/，无法作为关键字".to_string();
                break;
            }
        }

        col
    }

    /// 构成关键字表格
    pub fn excel_data(&self) -> Vec<ColData> {
        (self.first_col..=self.last_col)
            .map(|c| self.col_data(c))
            .collect()
    }
}
