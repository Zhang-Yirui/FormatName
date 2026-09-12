use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;

use calamine::{Cell, Data, Range, Reader, Xlsx};
use regex::Regex;
use serde_json::Value;

use crate::model::ColData;

/// 允许读取的表格扩展名
pub const ALLOWED_EXTS: [&str; 4] = ["xlsx", "xlsm", "xltx", "xltm"];
/// 允许读取的文本表格扩展名：内容按 CSV / JSON 解析
pub const TEXT_EXTS: [&str; 3] = ["csv", "json", "txt"];

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

/// 去掉 UTF-8 BOM
fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// 拆分 Markdown 表格的一行，去掉首尾多余的竖线
fn split_md_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let mut cells: Vec<String> = trimmed.split('|').map(|c| c.trim().to_string()).collect();
    if trimmed.starts_with('|') && cells.len() > 1 {
        cells.remove(0);
    }
    if trimmed.ends_with('|') && cells.len() > 1 {
        cells.pop();
    }
    cells
}

/// 是否为 Markdown 表格的分隔行，例如 `| --- | :---: |`
fn is_md_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return false;
    }
    let cells = split_md_row(trimmed);
    !cells.is_empty()
        && cells.iter().all(|c| {
        let c = c.trim_matches(':');
        !c.is_empty() && c.chars().all(|ch| ch == '-')
    })
}

/// 猜测分隔符：优先制表符（从 Excel / WPS 复制出来的表格），其次逗号、分号
fn detect_delimiter(lines: &[&str]) -> char {
    let sample = lines.first().unwrap_or(&"");
    if sample.contains('\t') {
        '\t'
    } else if sample.contains(',') {
        ','
    } else if sample.contains(';') {
        ';'
    } else {
        '\t'
    }
}

/// 按分隔符解析文本，支持引号包裹（引号内的分隔符与换行不生效）
fn parse_delimited(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                // 两个连续的引号表示一个普通的引号
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => in_quotes = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' => in_quotes = true,
            '\r' => {}
            '\n' => {
                row.push(field.trim().to_string());
                field.clear();
                rows.push(std::mem::take(&mut row));
            }
            c if c == delimiter => {
                row.push(field.trim().to_string());
                field.clear();
            }
            c => field.push(c),
        }
    }
    row.push(field.trim().to_string());
    rows.push(row);

    rows.retain(|r| r.iter().any(|c| !c.is_empty()));
    rows
}

/// 解析 Markdown 表格：分隔行前的一行是表头，其后是数据
fn parse_markdown(text: &str) -> Vec<Vec<String>> {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let separator = lines.iter().position(|l| is_md_separator(l));

    let mut rows: Vec<Vec<String>> = match separator {
        // 分隔行前面那一行是表头（表格上方的标题行会被忽略）
        Some(i) if i > 0 => {
            let mut rows = vec![split_md_row(lines[i - 1])];
            rows.extend(
                lines[i + 1..]
                    .iter()
                    .filter(|l| !is_md_separator(l))
                    .map(|l| split_md_row(l)),
            );
            rows
        }
        // 第一行就是分隔行：没有表头，剩下的都当作数据
        Some(_) => lines.iter().skip(1).map(|l| split_md_row(l)).collect(),
        // 没有分隔行：第一行当作表头
        None => lines.iter().map(|l| split_md_row(l)).collect(),
    };

    rows.retain(|r| r.iter().any(|c| !c.is_empty()));
    rows
}

/// JSON 值转单元格文本：整数不带小数点，null 记为空
fn json_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.to_string(),
            (None, Some(f)) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            (None, Some(f)) => f.to_string(),
            (None, None) => n.to_string(),
        },
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// 解析 JSON：支持对象数组 `[{...}]`、二维数组 `[["a","b"], ...]` 与单个对象
fn parse_json(text: &str) -> Option<Vec<Vec<String>>> {
    let value: Value = serde_json::from_str(text).ok()?;

    // 对象（数组）：出现过的键按先后顺序作为表头
    let objects: Vec<&serde_json::Map<String, Value>> = match &value {
        Value::Object(obj) => vec![obj],
        Value::Array(items) => items.iter().filter_map(|v| v.as_object()).collect(),
        _ => return None,
    };
    if let Value::Array(items) = &value {
        if objects.len() != items.len() {
            // 二维数组：每一项都是一行
            let rows: Vec<Vec<String>> = items
                .iter()
                .map(|v| match v {
                    Value::Array(cells) => cells.iter().map(json_to_string).collect(),
                    other => vec![json_to_string(other)],
                })
                .collect();
            return Some(rows);
        }
    }

    let mut headers: Vec<String> = Vec::new();
    for obj in &objects {
        for key in obj.keys() {
            if !headers.contains(key) {
                headers.push(key.clone());
            }
        }
    }
    let mut rows = vec![headers.clone()];
    for obj in &objects {
        rows.push(
            headers
                .iter()
                .map(|h| obj.get(h).map(json_to_string).unwrap_or_default())
                .collect(),
        );
    }
    Some(rows)
}

/// 解析文本表格，自动识别 JSON、Markdown 表格与 CSV / TSV
///
/// 第一行为表头，其余为数据行
pub fn parse_text_table(text: &str) -> Vec<Vec<String>> {
    let text = strip_bom(text);
    let trimmed = text.trim();
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        if let Some(rows) = parse_json(trimmed) {
            return rows;
        }
    }
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    let is_md = lines.iter().any(|l| l.trim().starts_with('|'))
        || lines.get(1).is_some_and(|l| is_md_separator(l));

    if is_md {
        parse_markdown(text)
    } else {
        parse_delimited(text, detect_delimiter(&lines))
    }
}

/// 把二维文本表格转成 calamine 的 Range，这样后面的分析与 Excel 完全一致
pub fn range_from_rows(rows: &[Vec<String>]) -> Range<Data> {
    let mut cells: Vec<Cell<Data>> = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, value) in row.iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            cells.push(Cell::new((r as u32, c as u32), Data::String(value.clone())));
        }
    }
    Range::from_sparse(cells)
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

/// 从一段文本（CSV / Markdown / 粘贴的表格）构建工作表
pub fn sheet_from_text(text: &str) -> SheetTable {
    SheetTable::new(range_from_rows(&parse_text_table(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CSV：逗号分隔 + CRLF 换行
    #[test]
    fn parse_csv_table() {
        let mut sheet = sheet_from_text("姓名,学号,班级\r\n张三,2021001,计科1班\r\n李四,2021002,计科2班\r\n");
        assert_eq!(sheet.is_correct(), 1);
        let data = sheet.excel_data();
        assert_eq!(data.len(), 3);
        assert_eq!(data[0].key, "姓名");
        assert_eq!(data[1].values, vec!["2021001", "2021002"]);
    }

    /// Markdown 表格：分隔行带对齐冒号，单元格可以为空
    #[test]
    fn parse_markdown_table() {
        let mut sheet = sheet_from_text(
            "| 姓名 | 学号 | 备注 |\n| --- | :---: | --- |\n| 张三 | 1 | \"a,b\" |\n| 李四 | 2 |  |\n",
        );
        assert_eq!(sheet.is_correct(), 1);
        let data = sheet.excel_data();
        assert_eq!(data.len(), 3);
        assert_eq!(data[0].key, "姓名");
        assert_eq!(data[2].values, vec!["\"a,b\"", ""]);
    }

    /// JSON：对象数组取所有键做表头，二维数组直接作为行
    #[test]
    fn parse_json_table() {
        let mut sheet = sheet_from_text(
            r#"[{"姓名":"张三","学号":2021001},{"姓名":"李四","学号":2021002,"备注":null}]"#,
        );
        assert_eq!(sheet.is_correct(), 1);
        let data = sheet.excel_data();
        assert_eq!(data.iter().map(|c| c.key.clone()).collect::<Vec<_>>(), vec!["姓名", "学号", "备注"]);
        assert_eq!(data[1].values, vec!["2021001", "2021002"]);
        assert_eq!(data[2].values, vec!["", ""]);

        let mut sheet = sheet_from_text(r#"[["姓名","学号"],["张三","1"]]"#);
        assert_eq!(sheet.is_correct(), 1);
        let data = sheet.excel_data();
        assert_eq!(data[0].key, "姓名");
        assert_eq!(data[1].values, vec!["1"]);
    }

    /// 从表格软件复制出来的表格：制表符分隔，引号里的内容保持原样
    #[test]
    fn parse_tsv_table() {
        let mut sheet = sheet_from_text("姓名\t备注\n张三\t\"含\t制表符\"\n李四\t普通\n");
        assert_eq!(sheet.is_correct(), 1);
        let data = sheet.excel_data();
        assert_eq!(data[1].values, vec!["含\t制表符", "普通"]);
    }
}
