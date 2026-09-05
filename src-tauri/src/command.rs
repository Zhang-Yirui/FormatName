use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use regex::Regex;
use tauri::State;

use crate::excel::{invalid_char_re, load_range, SheetTable, ALLOWED_EXTS};
use crate::model::{
    ApiResp, AppInfosResp, ColData, Config, ExecuteItem, ExecuteResp, NamePair,
};
use crate::state::AppState;

/// 去掉路径两侧可能出现的引号
fn trim_path(path: &str) -> String {
    path.trim().trim_matches('"').trim().to_string()
}

/// 取文件名后缀（包含小数点）
fn last_name(file_name: &str) -> String {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\.[^.]*$").unwrap());
    match re.find(file_name) {
        Some(m) => m.as_str().to_string(),
        None => String::new(),
    }
}

/// 把值编译成正则，非法正则退化为普通文本匹配
fn compile(value: &str) -> Regex {
    match Regex::new(value) {
        Ok(re) => re,
        Err(_) => Regex::new(&regex::escape(value)).unwrap(),
    }
}

// -------------------------------- 基础接口 --------------------------------
#[tauri::command]
pub fn get_app_info() -> AppInfosResp {
    AppInfosResp::from_cargo()
}

/// 获取当前程序的数据（Excel 解析后的列数据）
#[tauri::command]
pub fn get_data(state: State<AppState>) -> Vec<ColData> {
    state.get_config().data
}

/// 清空已导入的数据（重新选择 Excel）
#[tauri::command]
pub fn clear_data(state: State<AppState>) -> ApiResp {
    if let Err(e) = state.set_config(Config::default()) {
        return ApiResp::err(1, e);
    }
    *state.execute.lock().unwrap() = Default::default();
    ApiResp::ok("已清空")
}

// ---------------------------------------------------------------- 第一步：Excel

/// 提交 excel 的路径
///
/// 返回值：
/// - 0：读取成功
/// - 1：文件不存在
/// - 2：文件扩展名不对
/// - 3：文件为空
/// - 4：文件少于 2 行
/// - 5：表头存在重复
/// - 6：文件无法解析
#[tauri::command]
pub fn submit_excel_path(path: String, state: State<AppState>) -> ApiResp {
    let path = trim_path(&path);
    let file = PathBuf::from(&path);

    if !file.exists() || !file.is_file() {
        return ApiResp::err(1, "提交失败，文件不存在");
    }
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXTS.contains(&ext.as_str()) {
        return ApiResp::err(
            2,
            "提交失败，不能读取该格式文件，请选择(.xlsx)(.xlsm)(.xltx)(.xltm)文件",
        );
    }

    let range = match load_range(&path) {
        Ok(r) => r,
        Err(e) => return ApiResp::err(6, e),
    };
    let mut sheet = SheetTable::new(range);
    let code = sheet.is_correct();

    match code {
        1 => {
            let config = Config {
                data: sheet.excel_data(),
            };
            if let Err(e) = state.set_config(config) {
                return ApiResp::err(6, e);
            }
            *state.execute.lock().unwrap() = Default::default();
            ApiResp::ok("读取成功")
        }
        0 => ApiResp::err(3, "文件不能为空"),
        -1 => ApiResp::err(4, "文件应至少有2行"),
        2 => ApiResp::err(5, "文件表头存在重复"),
        _ => ApiResp::err(6, "无法识别的表格格式"),
    }
}

// ---------------------------------------------------------------- 第二步：关键字

/// 提交关键字配置
#[tauri::command]
pub fn submit_data(data: Vec<ColData>, state: State<AppState>) -> ApiResp {
    let config = Config { data };
    match state.set_config(config) {
        Ok(_) => ApiResp::ok("上传成功"),
        Err(e) => ApiResp::err(1, e),
    }
}

// ---------------------------------------------------------------- 第三步：命名格式

/// 接收要改名的文件夹路径并生成新旧名字对照
///
/// 返回值：
/// - 0：提交成功
/// - 1：路径不存在
/// - 2：路径为文件路径
/// - 3：路径包含了自身程序
/// - 4：尚未导入 Excel 数据
/// - 5：新文件名中会包含不允许使用的字符
/// - 6：目录读取失败
#[tauri::command]
pub fn submit_execute(path: String, execute: Vec<ExecuteItem>, state: State<AppState>) -> ApiResp {
    let path = trim_path(&path);
    let dir = PathBuf::from(&path);

    if !dir.exists() {
        return ApiResp::err(1, "提交失败，目录不存在");
    }
    if !dir.is_dir() {
        return ApiResp::err(2, "提交失败，请提交一个目录而非文件");
    }
    if state.app_dir.starts_with(&dir) {
        return ApiResp::err(3, "提交失败，请不要提交包含本程序的路径");
    }

    let data = state.get_config().data;
    if data.is_empty() {
        return ApiResp::err(4, "请先导入 Excel 数据");
    }

    let old = match list_files(&dir) {
        Ok(v) => v,
        Err(e) => return ApiResp::err(6, format!("读取目录失败：{}", e)),
    };

    // 为关键字排序：重复次数越少优先级越高
    let mut keywords: Vec<ColData> = data.iter().filter(|c| c.is_key_word).cloned().collect();
    keywords.sort_by_key(|c| c.delta);

    // 生成新名字列表
    let rows = data[0].values.len();
    let mut new: Vec<String> = Vec::with_capacity(rows);
    for i in 0..rows {
        let mut name = String::new();
        for item in &execute {
            match item {
                ExecuteItem::Index(j) => {
                    let value = data
                        .get(*j)
                        .and_then(|c| c.values.get(i))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if value.is_empty() {
                        name.push_str("空值");
                    } else {
                        name.push_str(value);
                    }
                }
                ExecuteItem::Null(()) => {}
                ExecuteItem::Text(t) => name.push_str(t),
            }
        }
        new.push(name);
    }

    // 匹配新旧名字
    let mut map = vec![0i32; rows];
    let mut list: Vec<NamePair> = Vec::new();
    for old_name in &old {
        let suffix = last_name(old_name);
        let mut found = false;
        for kw in &keywords {
            for (k, value) in kw.values.iter().enumerate() {
                if value.is_empty() || k >= new.len() {
                    continue;
                }
                if compile(value).is_match(old_name) {
                    let new_name = if map[k] > 0 {
                        format!("{}({}){}", new[k], map[k], suffix)
                    } else {
                        format!("{}{}", new[k], suffix)
                    };
                    list.push(NamePair {
                        old: old_name.clone(),
                        new: new_name,
                    });
                    map[k] += 1;
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
    }

    // 查询新名字中是否包含非法字符
    for name in &new {
        if invalid_char_re().is_match(name) {
            return ApiResp::err(5, "新文件名中会包含不允许使用的字符");
        }
    }

    let mut guard = state.execute.lock().unwrap();
    *guard = crate::model::Execute {
        flag: 0,
        path: path.clone(),
        execute,
        data: keywords,
        new,
        map,
        old,
        list,
    };
    drop(guard);

    ApiResp::ok("提交成功")
}

fn list_files(dir: &Path) -> Result<Vec<String>, String> {
    let mut files: Vec<String> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    files.sort();
    Ok(files)
}

// ---------------------------------------------------------------- 第四步：执行

/// 获取本次改名的操作以及新旧名字
#[tauri::command]
pub fn get_execute(state: State<AppState>) -> ExecuteResp {
    let guard = state.execute.lock().unwrap();
    ExecuteResp {
        map: guard.map.clone(),
        list: guard.list.clone(),
        new: guard.new.clone(),
        old: guard.old.clone(),
        flag: guard.flag,
    }
}

/// 发起重命名
///
/// 返回值：
/// - 0：改名成功
/// - 1：重复点击改名
/// - 2：新名字不能重复
/// - 3：部分文件改名失败
#[tauri::command]
pub fn rename(state: State<AppState>) -> ApiResp {
    let mut guard = state.execute.lock().unwrap();
    if guard.flag != 0 {
        return ApiResp::err(1, "已经改过名了，请勿重复点击");
    }
    if guard.list.is_empty() {
        return ApiResp::err(2, "没有需要改名的文件，请先提交目录");
    }

    let mut uniq: HashSet<&String> = HashSet::new();
    for pair in &guard.list {
        if !uniq.insert(&pair.new) {
            return ApiResp::err(2, "新名字不能重复");
        }
    }

    let dir = PathBuf::from(&guard.path);
    let pairs = guard.list.clone();

    let mut failed: Vec<String> = Vec::new();
    for pair in &pairs {
        if let Err(_) = fs::rename(dir.join(&pair.old), dir.join(&pair.new)) {
            failed.push(pair.old.clone());
        }
    }
    // 有的旧名字可能和某些新名字相同，先改成一个临时名字再改成目标名字
    if !failed.is_empty() {
        let retry = failed;
        failed = Vec::new();
        for old in &retry {
            let pair = pairs.iter().find(|p| &p.old == old).unwrap();
            let tmp = format!("__format_name_tmp__{}", pair.new);
            let from = dir.join(&pair.old);
            let mid = dir.join(&tmp);
            let to = dir.join(&pair.new);
            match fs::rename(&from, &mid).and_then(|_| fs::rename(&mid, &to)) {
                Ok(_) => {}
                Err(_) => {
                    let _ = fs::rename(&mid, &from);
                    failed.push(pair.old.clone());
                }
            }
        }
    }

    if !failed.is_empty() {
        return ApiResp::err(3, format!("以下文件改名失败：{}", failed.join("、")));
    }

    guard.flag = 1;
    ApiResp::ok("改名成功")
}

/// 恢复原文件名
///
/// 返回值：
/// - 0：恢复成功
/// - 1：尚未进行重命名
/// - 2：部分文件恢复失败
#[tauri::command]
pub fn recover(state: State<AppState>) -> ApiResp {
    let mut guard = state.execute.lock().unwrap();
    if guard.flag == 0 {
        return ApiResp::err(1, "尚未进行重命名");
    }

    let dir = PathBuf::from(&guard.path);
    let pairs = guard.list.clone();

    let mut failed: Vec<String> = Vec::new();
    for pair in &pairs {
        if let Err(_) = fs::rename(dir.join(&pair.new), dir.join(&pair.old)) {
            failed.push(pair.new.clone());
        }
    }
    if !failed.is_empty() {
        let retry = failed;
        failed = Vec::new();
        for new in &retry {
            let pair = pairs.iter().find(|p| &p.new == new).unwrap();
            let tmp = format!("__format_name_tmp__{}", pair.old);
            let from = dir.join(&pair.new);
            let mid = dir.join(&tmp);
            let to = dir.join(&pair.old);
            match fs::rename(&from, &mid).and_then(|_| fs::rename(&mid, &to)) {
                Ok(_) => {}
                Err(_) => {
                    let _ = fs::rename(&mid, &from);
                    failed.push(pair.new.clone());
                }
            }
        }
    }

    if !failed.is_empty() {
        return ApiResp::err(2, format!("以下文件恢复失败：{}", failed.join("、")));
    }

    guard.flag = 0;
    ApiResp::ok("恢复成功")
}

/// 备份当前目录中的文件
///
/// 返回值：
/// - 0：备份成功
/// - 1：已经改过名字了
/// - 2：目录已存在
/// - 3：备份失败
#[tauri::command]
pub fn backup(state: State<AppState>) -> ApiResp {
    let guard = state.execute.lock().unwrap();
    if guard.flag != 0 {
        return ApiResp::err(1, "已经改名请先恢复再备份");
    }
    if guard.path.is_empty() {
        return ApiResp::err(3, "请先提交要改名的文件夹");
    }

    let now = Local::now();
    let name = format!("{} 备份", now.format("%Y年%m月%d日 %H时%M分%S"));
    let base = PathBuf::from(&guard.path);
    let dir = base.join(&name);
    if dir.exists() {
        return ApiResp::err(2, "目录已存在，请重试");
    }
    if let Err(e) = fs::create_dir_all(&dir) {
        return ApiResp::err(3, format!("创建备份目录失败：{}", e));
    }
    for file in &guard.old {
        let from = base.join(file);
        let to = dir.join(file);
        if let Err(e) = fs::copy(&from, &to) {
            return ApiResp::err(3, format!("备份文件 {} 失败：{}", file, e));
        }
    }
    ApiResp::ok("备份成功")
}
