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
    if let Err(e) = state.reset_execute() {
        return ApiResp::err(1, e);
    }
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
            if let Err(e) = state.reset_execute() {
                return ApiResp::err(6, e);
            }
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

    let (keywords, new, map, list) = match build_execute(&data, &execute, &old) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let exec = crate::model::Execute {
        flag: 0,
        path: path.clone(),
        execute,
        data: keywords,
        new,
        map,
        old,
        list,
    };
    if let Err(e) = state.set_execute(exec) {
        return ApiResp::err(6, e);
    }

    ApiResp::ok("提交成功")
}

/// 用关键字匹配一个文件名，返回它对应的新名字下标
fn find_key(keywords: &[ColData], new_len: usize, file_name: &str) -> Option<usize> {
    for kw in keywords {
        for (k, value) in kw.values.iter().enumerate() {
            if value.is_empty() || k >= new_len {
                continue;
            }
            if compile(value).is_match(file_name) {
                return Some(k);
            }
        }
    }
    None
}

/// 根据表格数据、命名格式和目录中的文件名，计算新名字与新旧名字对照
///
/// 返回 `(排序后的关键字, 新名字列表, 每个名字的匹配次数, 新旧名字对照表)`
fn build_execute(
    data: &[ColData],
    execute: &[ExecuteItem],
    old: &[String],
) -> Result<(Vec<ColData>, Vec<String>, Vec<i32>, Vec<NamePair>), ApiResp> {
    // 为关键字排序：重复次数越少优先级越高
    let mut keywords: Vec<ColData> = data.iter().filter(|c| c.is_key_word).cloned().collect();
    keywords.sort_by_key(|c| c.delta);

    // 生成新名字列表
    let rows = data[0].values.len();
    let mut new: Vec<String> = Vec::with_capacity(rows);
    for i in 0..rows {
        let mut name = String::new();
        for item in execute {
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
    for old_name in old {
        let Some(k) = find_key(&keywords, new.len(), old_name) else {
            continue;
        };
        let suffix = last_name(old_name);
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
    }

    // 查询新名字中是否包含非法字符
    for name in &new {
        if invalid_char_re().is_match(name) {
            return Err(ApiResp::err(5, "新文件名中会包含不允许使用的字符"));
        }
    }

    Ok((keywords, new, map, list))
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
    let exec = state.get_execute();
    ExecuteResp {
        map: exec.map,
        list: exec.list,
        new: exec.new,
        old: exec.old,
        flag: exec.flag,
    }
}

/// 重新扫描目录并重新计算新旧名字对照
///
/// 目录里的文件在程序之外被增删改后，用它刷新分析结果
///
/// - 未改名（`flag == 0`）：目录里都是旧文件名，整份重新计算；
/// - 已改名（`flag == 1`）：目录里是新文件名，只做增量更新——
///   已经不在目录里的文件从对照表里移除，新出现的文件立即按规则改名并加入对照表，
///   这样对照表始终是「本次已完成的改名映射」，因此随时都能正常恢复
///
/// 返回值：
/// - 0：刷新成功
/// - 3：尚未提交目录、目录已不存在，或新增文件改名失败（已自动回滚）
/// - 4：尚未导入 Excel 数据
/// - 5：新文件名中会包含不允许使用的字符
/// - 6：目录读取失败
/// - 7：状态保存失败
#[tauri::command]
pub fn rescan(state: State<AppState>) -> ApiResp {
    let mut exec = state.get_execute();
    if exec.path.is_empty() {
        return ApiResp::err(3, "请先提交要改名的文件夹");
    }
    let dir = PathBuf::from(&exec.path);
    if !dir.is_dir() {
        return ApiResp::err(3, "目录已不存在，请重新提交文件夹");
    }

    let current = match list_files(&dir) {
        Ok(v) => v,
        Err(e) => return ApiResp::err(6, format!("读取目录失败：{}", e)),
    };

    // ------------------------------------------------ 未改名：整份重算
    if exec.flag == 0 {
        let data = state.get_config().data;
        if data.is_empty() {
            return ApiResp::err(4, "请先导入 Excel 数据");
        }

        let (keywords, new, map, list) = match build_execute(&data, &exec.execute, &current) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let total = current.len();
        let matched = list.len();

        exec.data = keywords;
        exec.new = new;
        exec.map = map;
        exec.old = current;
        exec.list = list;
        if let Err(e) = state.set_execute(exec) {
            return ApiResp::err(7, e);
        }

        return ApiResp::ok(format!(
            "已重新扫描：共 {} 个文件，匹配到 {} 个",
            total, matched
        ));
    }

    // ------------------------------------------------ 已改名：增量更新
    // 磁盘上当前存在的文件名，给新文件挑名字时用来避让
    let mut exists: HashSet<String> = current.iter().cloned().collect();
    // 已经纳入本次改名任务的文件名
    let mut used: HashSet<String> = HashSet::new();

    let mut map = vec![0i32; exec.new.len()];
    let mut list: Vec<NamePair> = Vec::new();
    let mut kept = 0usize;
    let mut gone = 0usize;

    // 还在的就保留原来的映射，已经不在目录里的从对照表中移除
    for pair in &exec.list {
        if !exists.contains(&pair.new) {
            gone += 1;
            continue;
        }
        if let Some(k) = find_key(&exec.data, exec.new.len(), &pair.old) {
            map[k] += 1;
        }
        used.insert(pair.new.clone());
        list.push(pair.clone());
        kept += 1;
    }

    // 新出现的文件：按规则改名后加入对照表
    let mut jobs: Vec<(String, String)> = Vec::new();
    for name in &current {
        if used.contains(name) {
            continue;
        }
        let Some(k) = find_key(&exec.data, exec.new.len(), name) else {
            continue;
        };
        let suffix = last_name(name);
        let mut n = map[k];
        let mut new_name = if n > 0 {
            format!("{}({}){}", exec.new[k], n, suffix)
        } else {
            format!("{}{}", exec.new[k], suffix)
        };
        while exists.contains(&new_name) {
            n += 1;
            new_name = format!("{}({}){}", exec.new[k], n, suffix);
        }
        exists.insert(new_name.clone());
        jobs.push((name.clone(), new_name));
    }

    let jobs_ref: Vec<(&str, &str)> = jobs
        .iter()
        .map(|(old, new)| (old.as_str(), new.as_str()))
        .collect();
    if !jobs_ref.is_empty() {
        if let Err(e) = rename_atomic(&dir, &jobs_ref) {
            return ApiResp::err(3, rename_err_msg("改名", e));
        }
        for (old, new) in &jobs {
            if let Some(k) = find_key(&exec.data, exec.new.len(), old) {
                map[k] += 1;
            }
            list.push(NamePair {
                old: old.clone(),
                new: new.clone(),
            });
        }
    }

    exec.map = map;
    exec.old = current;
    exec.list = list;
    if let Err(e) = state.set_execute(exec) {
        return ApiResp::err(7, e);
    }

    let mut msg = format!("已重新扫描：{} 个文件保持已改名状态", kept);
    if gone > 0 {
        msg += &format!("，{} 个文件已不在目录中，已从对照表移除", gone);
    }
    if !jobs.is_empty() {
        msg += &format!("，新增 {} 个文件并已改名", jobs.len());
    }
    ApiResp::ok(msg)
}

/// 发起重命名
///
/// 改名过程保证原子性：只要有文件改不动，已经改过的文件会被改回原样。
///
/// 返回值：
/// - 0：改名成功
/// - 1：重复点击改名
/// - 2：新名字不能重复
/// - 3：部分文件改名失败（已自动回滚）
/// - 7：状态保存失败
#[tauri::command]
pub fn rename(state: State<AppState>) -> ApiResp {
    let exec = state.get_execute();
    if exec.flag != 0 {
        return ApiResp::err(1, "已经改过名了，请勿重复点击");
    }
    if exec.list.is_empty() {
        return ApiResp::err(2, "没有需要改名的文件，请先提交目录");
    }

    let mut uniq: HashSet<&String> = HashSet::new();
    for pair in &exec.list {
        if !uniq.insert(&pair.new) {
            return ApiResp::err(2, "新名字不能重复");
        }
    }

    let dir = PathBuf::from(&exec.path);
    let jobs: Vec<(&str, &str)> = exec
        .list
        .iter()
        .map(|p| (p.old.as_str(), p.new.as_str()))
        .collect();
    if let Err(e) = rename_atomic(&dir, &jobs) {
        return ApiResp::err(3, rename_err_msg("改名", e));
    }

    if let Err(e) = state.set_flag(1) {
        return ApiResp::err(7, e);
    }
    ApiResp::ok("改名成功")
}

/// 恢复原文件名
///
/// 和改名一样保证原子性：只要有文件恢复不了，已经恢复的文件会被改回新名字。
///
/// 返回值：
/// - 0：恢复成功
/// - 1：尚未进行重命名
/// - 2：部分文件恢复失败（已自动回滚）
/// - 7：状态保存失败
#[tauri::command]
pub fn recover(state: State<AppState>) -> ApiResp {
    let exec = state.get_execute();
    if exec.flag == 0 {
        return ApiResp::err(1, "尚未进行重命名");
    }

    let dir = PathBuf::from(&exec.path);
    let jobs: Vec<(&str, &str)> = exec
        .list
        .iter()
        .map(|p| (p.new.as_str(), p.old.as_str()))
        .collect();
    if let Err(e) = rename_atomic(&dir, &jobs) {
        return ApiResp::err(2, rename_err_msg("恢复", e));
    }

    if let Err(e) = state.set_flag(0) {
        return ApiResp::err(7, e);
    }
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
    let exec = state.get_execute();
    if exec.flag != 0 {
        return ApiResp::err(1, "已经改名请先恢复再备份");
    }
    if exec.path.is_empty() {
        return ApiResp::err(3, "请先提交要改名的文件夹");
    }

    let now = Local::now();
    let name = format!("{} 备份", now.format("%Y年%m月%d日 %H时%M分%S"));
    let base = PathBuf::from(&exec.path);
    let dir = base.join(&name);
    if dir.exists() {
        return ApiResp::err(2, "目录已存在，请重试");
    }
    if let Err(e) = fs::create_dir_all(&dir) {
        return ApiResp::err(3, format!("创建备份目录失败：{}", e));
    }
    for file in &exec.old {
        let from = base.join(file);
        let to = dir.join(file);
        if let Err(e) = fs::copy(&from, &to) {
            return ApiResp::err(3, format!("备份文件 {} 失败：{}", file, e));
        }
    }
    ApiResp::ok("备份成功")
}

// ---------------------------------------------------------------- 文件操作

/// 一次批量重命名中没能完成的部分
#[derive(Debug)]
struct RenameErr {
    /// 没能改成目标名字的文件（操作前的名字）
    failed: Vec<String>,
    /// 回滚时没能改回去的文件（操作后的名字）
    rollback_failed: Vec<String>,
    /// 补充说明，例如「目标名字已被占用」
    note: String,
}

impl RenameErr {
    fn new(failed: Vec<String>, note: &str) -> Self {
        Self {
            failed,
            rollback_failed: Vec::new(),
            note: note.to_string(),
        }
    }
}

/// 一次改名任务在磁盘上的当前状态
struct Task<'a> {
    /// 改动之前的名字
    from: &'a str,
    /// 想要改成的名字
    to: &'a str,
    /// 中转用的临时名字
    tmp: String,
    /// 此刻在磁盘上的名字
    now: String,
}

/// 目录内的一次重命名，源名和目标名相同时什么都不用做
fn move_file(dir: &Path, from: &str, to: &str) -> std::io::Result<()> {
    if from == to {
        return Ok(());
    }
    fs::rename(dir.join(from), dir.join(to))
}

/// 两个文件名是否指向同一个文件
///
/// Windows 的文件名不区分大小写，所以 `a.txt` 和 `A.txt` 算同一个文件，
/// 只改大小写时不能当成「目标名字已被占用」。
fn same_name(a: &str, b: &str) -> bool {
    a == b || (cfg!(windows) && a.eq_ignore_ascii_case(b))
}

/// 挑一个目录里没有的临时名字（避开上次异常退出留下的残留）
fn pick_tmp(dir: &Path, index: usize) -> String {
    let mut name = format!("__format_name_tmp__{}", index);
    let mut n = 0;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("__format_name_tmp__{}_{}", index, n);
    }
    name
}

/// 批量重命名，保证原子性
///
/// 按给定顺序把 `from` 改名为 `to`，中途出错会把已经改过的文件全部改回原样：
///
/// 1. 先检查目标名字：不能重复，也不能是目录里与本次无关的同名文件
///    （`fs::rename` 在 Windows 上会直接覆盖同名文件，必须提前拦住，否则会丢文件）；
/// 2. 每个文件先改成一个不会重复的临时名字，此时所有旧名字都被腾空；
/// 3. 再从临时名字改成目标名字，这样即使旧名字和别人的新名字相同也不会互相覆盖；
/// 4. 任何一步失败都调用 `rollback` 把已改动的文件改回去。
fn rename_atomic(dir: &Path, jobs: &[(&str, &str)]) -> Result<(), RenameErr> {
    // 参与本次改名的旧名字
    let sources: HashSet<&str> = jobs.iter().map(|job| job.0).collect();
    let mut targets: HashSet<&str> = HashSet::new();
    for &(from, to) in jobs {
        if from == to {
            continue;
        }
        if !targets.insert(to) {
            return Err(RenameErr::new(
                vec![from.to_string()],
                "目标名字与其他文件重复",
            ));
        }
        // 目标名字被「不参与本次改名」的文件占着时不能覆盖，直接报错
        if !sources.iter().any(|from| same_name(from, to)) && dir.join(to).exists() {
            return Err(RenameErr::new(
                vec![from.to_string()],
                &format!("目录里已经存在名为 {} 的文件", to),
            ));
        }
    }

    let mut tasks: Vec<Task> = jobs
        .iter()
        .enumerate()
        .map(|(i, &(from, to))| Task {
            from,
            to,
            tmp: pick_tmp(dir, i),
            now: from.to_string(),
        })
        .collect();

    // 第一步：旧名字 -> 临时名字
    for i in 0..tasks.len() {
        if tasks[i].from == tasks[i].to {
            continue;
        }
        let tmp = tasks[i].tmp.clone();
        if move_file(dir, &tasks[i].now, &tmp).is_err() {
            let mut err = RenameErr::new(vec![tasks[i].from.to_string()], "文件被占用或已不存在");
            err.rollback_failed = rollback(dir, &mut tasks[..i]);
            return Err(err);
        }
        tasks[i].now = tmp;
    }

    // 第二步：临时名字 -> 新名字（旧名字此时已全部腾空，不会互相覆盖）
    for i in 0..tasks.len() {
        if tasks[i].from == tasks[i].to {
            continue;
        }
        if move_file(dir, &tasks[i].now, tasks[i].to).is_err() {
            let mut err = RenameErr::new(vec![tasks[i].from.to_string()], "文件被占用或已不存在");
            // i 之前的文件已经改成新名字，i 之后的还停在临时名字，都要回滚
            err.rollback_failed = rollback(dir, &mut tasks);
            return Err(err);
        }
        tasks[i].now = tasks[i].to.to_string();
    }

    Ok(())
}

/// 把已经改动过的文件改回原样，返回没能改回去的文件（改动后的名字）
fn rollback(dir: &Path, tasks: &mut [Task]) -> Vec<String> {
    fn back(dir: &Path, task: &mut Task) -> bool {
        if task.now == task.from {
            return true;
        }
        if move_file(dir, &task.now, task.from).is_ok() {
            task.now = task.from.to_string();
            true
        } else {
            false
        }
    }

    loop {
        let mut progressed = false;
        for task in tasks.iter_mut() {
            if back(dir, task) {
                progressed = true;
            }
        }
        if tasks.iter().all(|task| task.now == task.from) {
            break;
        }
        if progressed {
            continue;
        }
        // 剩下的互相挡着（比如两个文件交换名字），先全部挪到临时名字腾出位置
        for task in tasks.iter_mut() {
            if task.now == task.from {
                continue;
            }
            let tmp = task.tmp.clone();
            if move_file(dir, &task.now, &tmp).is_ok() {
                task.now = tmp;
            }
        }
        let mut done = false;
        for task in tasks.iter_mut() {
            if back(dir, task) {
                done = true;
            }
        }
        if !done {
            break;
        }
    }

    tasks
        .iter()
        .filter(|task| task.now != task.from)
        .map(|task| task.now.clone())
        .collect()
}

/// 拼接批量重命名的错误提示
fn rename_err_msg(action: &str, err: RenameErr) -> String {
    let mut msg = format!("以下文件{}失败：{}", action, err.failed.join("、"));
    if !err.note.is_empty() {
        msg.push_str(&format!("（{}）", err.note));
    }
    if err.rollback_failed.is_empty() {
        msg.push_str("（已自动回滚，文件保持操作前的状态）");
    } else {
        msg.push_str(&format!(
            "；以下文件未能回滚，请手动处理：{}",
            err.rollback_failed.join("、")
        ));
    }
    msg
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("format_name_test_{}_{}", tag, nanos))
    }

    fn touch(dir: &Path, name: &str) {
        std::fs::create_dir_all(dir).unwrap();
        // 文件内容写成自己的名字，方便判断有没有被覆盖
        std::fs::write(dir.join(name), name).unwrap();
    }

    fn read(dir: &Path, name: &str) -> String {
        std::fs::read_to_string(dir.join(name)).unwrap()
    }

    #[test]
    fn rename_all_files() {
        let dir = temp_dir("rename_all");
        touch(&dir, "a.txt");
        touch(&dir, "b.txt");
        rename_atomic(&dir, &[("a.txt", "A.txt"), ("b.txt", "B.txt")]).unwrap();
        assert_eq!(list_files(&dir).unwrap(), vec!["A.txt", "B.txt"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_chain_keeps_both_files() {
        let dir = temp_dir("rename_chain");
        touch(&dir, "a.txt");
        touch(&dir, "b.txt");
        // 旧名字正好是另一个任务的新名字，两个文件都必须留下
        rename_atomic(&dir, &[("a.txt", "b.txt"), ("b.txt", "c.txt")]).unwrap();
        assert_eq!(list_files(&dir).unwrap(), vec!["b.txt", "c.txt"]);
        assert_eq!(read(&dir, "b.txt"), "a.txt");
        assert_eq!(read(&dir, "c.txt"), "b.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_swap_files() {
        let dir = temp_dir("rename_swap");
        touch(&dir, "a.txt");
        touch(&dir, "b.txt");
        rename_atomic(&dir, &[("a.txt", "b.txt"), ("b.txt", "a.txt")]).unwrap();
        assert_eq!(read(&dir, "a.txt"), "b.txt");
        assert_eq!(read(&dir, "b.txt"), "a.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let dir = temp_dir("rename_overwrite");
        touch(&dir, "a.txt");
        touch(&dir, "占用.txt");
        // 目标名字被无关文件占着，不能覆盖它
        rename_atomic(&dir, &[("a.txt", "占用.txt")]).unwrap_err();
        assert_eq!(read(&dir, "占用.txt"), "占用.txt");
        assert_eq!(read(&dir, "a.txt"), "a.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_rolls_back_on_failure() {
        let dir = temp_dir("rename_rollback");
        touch(&dir, "a.txt");
        touch(&dir, "b.txt");
        touch(&dir, "占用.txt");
        // a.txt 能改成 A.txt，b.txt 会覆盖掉无关的「占用.txt」，整批应当失败并回滚
        let err = rename_atomic(&dir, &[("a.txt", "A.txt"), ("b.txt", "占用.txt")]).unwrap_err();
        assert_eq!(err.failed, vec!["b.txt"]);
        assert!(err.rollback_failed.is_empty());
        // 已经改成功的 a.txt 应被改回去，目录保持原样
        assert_eq!(
            list_files(&dir).unwrap(),
            vec!["a.txt", "b.txt", "占用.txt"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
