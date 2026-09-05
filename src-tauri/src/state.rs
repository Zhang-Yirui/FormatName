use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::db::Db;
use crate::model::{Config, Execute};

/// 全局状态：配置 + 当前改名任务
///
/// 配置与改名任务都会写入数据目录下的 sqlite 数据库，
/// 内存里只保留一份副本，程序重启后能接着上次的进度继续操作（尤其是「恢复原文件名」）。
pub struct AppState {
    /// 程序的数据目录（数据库存放位置）
    pub app_dir: PathBuf,
    /// 数据库连接
    db: Db,
    /// Excel 读取出来的数据
    config: Mutex<Config>,
    /// 本次改名任务
    execute: Mutex<Execute>,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取程序数据目录：{}", e))?;
        let db = Db::open(&dir)?;
        let config = db.load_config()?.unwrap_or_default();
        let execute = db.load_execute()?.unwrap_or_default();
        Ok(Self {
            app_dir: dir,
            db,
            config: Mutex::new(config),
            execute: Mutex::new(execute),
        })
    }

    /// 读取当前配置（返回克隆，避免长期占用锁）
    pub fn get_config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    /// 读取当前改名任务（返回克隆）
    pub fn get_execute(&self) -> Execute {
        self.execute.lock().unwrap().clone()
    }

    /// 写入配置并落盘
    pub fn set_config(&self, config: Config) -> Result<(), String> {
        self.db.save_config(&config)?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }

    /// 写入改名任务并落盘
    pub fn set_execute(&self, execute: Execute) -> Result<(), String> {
        self.db.save_execute(&execute)?;
        *self.execute.lock().unwrap() = execute;
        Ok(())
    }

    /// 清空改名任务（内存 + 数据库）
    pub fn reset_execute(&self) -> Result<(), String> {
        self.db.clear_execute()?;
        *self.execute.lock().unwrap() = Execute::default();
        Ok(())
    }

    /// 只更新「是否已改名」标记并落盘
    pub fn set_flag(&self, flag: i32) -> Result<(), String> {
        let execute = {
            let mut guard = self.execute.lock().unwrap();
            guard.flag = flag;
            guard.clone()
        };
        self.db.save_execute(&execute)
    }
}
