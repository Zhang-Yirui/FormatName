use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::model::{Config, Execute};

/// 数据库文件名，存放在程序的数据目录下
pub const DB_FILE: &str = "format_name.db";

/// 程序的全部持久化数据：Excel 配置 + 当前改名任务，共用同一个 sqlite 文件
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// 打开数据目录下的数据库，不存在则创建并建表
    pub fn open(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("无法创建程序数据目录：{}", e))?;
        let path = dir.join(DB_FILE);
        let conn = Connection::open(&path).map_err(|e| format!("打开数据库失败：{}", e))?;
        Self::init(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn init(conn: &Connection) -> Result<(), String> {
        // config / execute 都只保留一行（id 固定为 1）
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS config (
                id   INTEGER PRIMARY KEY CHECK (id = 1),
                data TEXT    NOT NULL
            );
            CREATE TABLE IF NOT EXISTS execute (
                id        INTEGER PRIMARY KEY CHECK (id = 1),
                flag      INTEGER NOT NULL,
                path      TEXT    NOT NULL,
                items     TEXT    NOT NULL,
                data      TEXT    NOT NULL,
                new_names TEXT    NOT NULL,
                old_names TEXT    NOT NULL,
                map       TEXT    NOT NULL,
                list      TEXT    NOT NULL
            );",
        )
            .map_err(|e| format!("初始化数据库失败：{}", e))
    }

    /// 读取配置，从未保存过时返回 `None`
    pub fn load_config(&self) -> Result<Option<Config>, String> {
        let conn = self.conn.lock().unwrap();
        let text: Option<String> = conn
            .query_row("SELECT data FROM config WHERE id = 1", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .map_err(|e| format!("读取配置失败：{}", e))?;
        match text {
            Some(t) => Ok(Some(from_json("配置", &t)?)),
            None => Ok(None),
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<(), String> {
        let text = to_json("配置", config)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO config (id, data) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![text],
        )
            .map_err(|e| format!("保存配置失败：{}", e))?;
        Ok(())
    }

    /// 读取上次未完成的改名任务，没有则返回 `None`
    pub fn load_execute(&self) -> Result<Option<Execute>, String> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT flag, path, items, data, new_names, old_names, map, list
                 FROM execute WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("读取改名任务失败：{}", e))?;

        let Some((flag, path, items, data, new_names, old_names, map, list)) = row else {
            return Ok(None);
        };
        Ok(Some(Execute {
            flag,
            path,
            execute: from_json("命名格式", &items)?,
            data: from_json("关键字", &data)?,
            new: from_json("新名字", &new_names)?,
            old: from_json("旧名字", &old_names)?,
            map: from_json("匹配次数", &map)?,
            list: from_json("新旧名字对照", &list)?,
        }))
    }

    pub fn save_execute(&self, execute: &Execute) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO execute (id, flag, path, items, data, new_names, old_names, map, list)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                 flag = excluded.flag, path = excluded.path, items = excluded.items,
                 data = excluded.data, new_names = excluded.new_names,
                 old_names = excluded.old_names, map = excluded.map, list = excluded.list",
            params![
                execute.flag,
                execute.path,
                to_json("命名格式", &execute.execute)?,
                to_json("关键字", &execute.data)?,
                to_json("新名字", &execute.new)?,
                to_json("旧名字", &execute.old)?,
                to_json("匹配次数", &execute.map)?,
                to_json("新旧名字对照", &execute.list)?,
            ],
        )
            .map_err(|e| format!("保存改名任务失败：{}", e))?;
        Ok(())
    }

    /// 丢弃已保存的改名任务
    pub fn clear_execute(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM execute WHERE id = 1", [])
            .map_err(|e| format!("清除改名任务失败：{}", e))?;
        Ok(())
    }
}

fn to_json<T: serde::Serialize>(name: &str, value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("{}序列化失败：{}", name, e))
}

fn from_json<T: serde::de::DeserializeOwned>(name: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{}解析失败：{}", name, e))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::model::{ColData, ExecuteItem, NamePair};

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("format_name_test_{}_{}", tag, nanos))
    }

    fn sample_config() -> Config {
        Config {
            data: vec![ColData {
                key: "学号".to_string(),
                values: vec!["001".to_string(), "002".to_string()],
                is_key_word: true,
                display: true,
                reason: String::new(),
                delta: 1,
            }],
        }
    }

    fn sample_execute() -> Execute {
        Execute {
            flag: 1,
            path: "D:/作业".to_string(),
            execute: vec![
                ExecuteItem::Index(0),
                ExecuteItem::Text("-".to_string()),
                ExecuteItem::Null(()),
            ],
            data: sample_config().data,
            new: vec!["张三".to_string()],
            map: vec![1],
            old: vec!["作业.docx".to_string()],
            list: vec![NamePair {
                old: "作业.docx".to_string(),
                new: "张三.docx".to_string(),
            }],
        }
    }

    #[test]
    fn config_round_trip() {
        let dir = temp_dir("config");
        let db = Db::open(&dir).unwrap();
        assert!(db.load_config().unwrap().is_none());
        db.save_config(&sample_config()).unwrap();
        let config = db.load_config().unwrap().unwrap();
        assert_eq!(config.data[0].key, "学号");
        // 再次保存应覆盖同一行而不是新增
        db.save_config(&Config::default()).unwrap();
        assert!(db.load_config().unwrap().unwrap().data.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn execute_round_trip() {
        let dir = temp_dir("execute");
        let db = Db::open(&dir).unwrap();
        assert!(db.load_execute().unwrap().is_none());
        db.save_execute(&sample_execute()).unwrap();
        let exec = db.load_execute().unwrap().unwrap();
        assert_eq!(exec.flag, 1);
        assert_eq!(exec.path, "D:/作业");
        assert_eq!(exec.list[0].new, "张三.docx");
        assert!(matches!(exec.execute[2], ExecuteItem::Null(())));
        db.clear_execute().unwrap();
        assert!(db.load_execute().unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
