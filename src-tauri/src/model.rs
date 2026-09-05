use serde::{Deserialize, Serialize};
use crate::{APP_NAME, VERSION, DESCRIPTION, AUTHORS, LICENSE, REPOSITORY, HOMEPAGE,};

/// 表格中的一列数据（对应 Python 版本 Excel_List.return_col_data 的返回值）
///
/// - `key`：表头，可作为关键字
/// - `values`：该列除表头以外的所有值
/// - `isKeyWord`：是否作为关键字
/// - `display`：该项是否允许出现在命名格式中
/// - `reason`：系统给出的原因说明
/// - `delta`：该列值的重复次数，用于给关键字排序（越小优先级越高）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColData {
    pub key: String,
    pub values: Vec<String>,
    #[serde(rename = "isKeyWord")]
    pub is_key_word: bool,
    pub display: bool,
    pub reason: String,
    pub delta: usize,
}

/// 持久化到 config.json 的配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub data: Vec<ColData>,
}

/// 命名格式中的一项
///
/// - `Index(n)`：取第 n 列的值
/// - `Null`：占位（不输出任何内容）
/// - `Text(s)`：自定义文本 / 分隔符
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExecuteItem {
    Index(usize),
    Null(()),
    Text(String),
}

/// 新旧名字对照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamePair {
    pub old: String,
    pub new: String,
}

/// 一次改名任务的全部状态
///
/// - `flag`：是否执行过改名，1 为已改名，0 为未改名
/// - `path`：要操作的目录
/// - `execute`：命名格式
/// - `data`：按优先级排序后的关键字
/// - `new`：新名字列表
/// - `map`：每个名字被匹配到的次数标记
/// - `old`：目录中的旧文件名
/// - `list`：新旧名字对照表
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Execute {
    #[serde(default)]
    pub flag: i32,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub execute: Vec<ExecuteItem>,
    #[serde(default)]
    pub data: Vec<ColData>,
    #[serde(default)]
    pub new: Vec<String>,
    #[serde(default)]
    pub map: Vec<i32>,
    #[serde(default)]
    pub old: Vec<String>,
    #[serde(default)]
    pub list: Vec<NamePair>,
}

/// 统一接口返回结构（沿用 Python 版本的 code/msg 约定）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResp {
    pub code: i32,
    pub msg: String,
}

impl ApiResp {
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            code: 0,
            msg: msg.into(),
        }
    }

    pub fn err(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
        }
    }
}

/// /GetExecute 的返回值
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResp {
    pub map: Vec<i32>,
    pub list: Vec<NamePair>,
    pub new: Vec<String>,
    pub old: Vec<String>,
    pub flag: i32,
}

/// /GetAppInfo 的返回值
#[derive(Debug, Clone, Serialize)]
pub struct AppInfosResp {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub repository: String,
    pub homepage: String,
}

impl AppInfosResp {
    /// 从 Cargo.toml 编译期环境变量构造
    pub fn from_cargo() -> Self {
        Self {
            name: APP_NAME.to_string(),
            version: VERSION.to_string(),
            description: DESCRIPTION.to_string(),
            authors: AUTHORS
                .split(':')
                .map(|s| s.to_string())
                .collect(),
            license: LICENSE.to_string(),
            repository: REPOSITORY.to_string(),
            homepage: HOMEPAGE.to_string(),
        }
    }
}
