use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::model::{Config, Execute};

/// 全局状态：配置 + 当前改名任务
pub struct AppState {
    /// 程序的数据目录（config.json 存放位置）
    pub app_dir: PathBuf,
    /// Excel 读取出来的数据
    pub config: Mutex<Config>,
    /// 本次改名任务
    pub execute: Mutex<Execute>,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取程序数据目录：{}", e))?;
        fs::create_dir_all(&dir).map_err(|e| format!("无法创建程序数据目录：{}", e))?;
        let config = Self::load(&dir);
        Ok(Self {
            app_dir: dir,
            config: Mutex::new(config),
            execute: Mutex::new(Execute::default()),
        })
    }

    fn load(dir: &Path) -> Config {
        let path = dir.join("config.json");
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str::<Config>(&text).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    /// 把配置写入 config.json
    pub fn save_config(&self, config: &Config) -> Result<(), String> {
        let path = self.app_dir.join("config.json");
        let text =
            serde_json::to_string_pretty(config).map_err(|e| format!("配置序列化失败：{}", e))?;
        fs::write(path, text).map_err(|e| format!("配置写入失败：{}", e))
    }

    /// 读取当前配置（返回克隆，避免长期占用锁）
    pub fn get_config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    /// 写入配置并落盘
    pub fn set_config(&self, config: Config) -> Result<(), String> {
        self.save_config(&config)?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }
}
