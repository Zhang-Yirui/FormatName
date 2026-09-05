use tauri::Manager;

mod command;
mod excel;
mod model;
mod state;

use state::AppState;

/// 程序名称
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
/// 程序版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// 程序描述
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
/// 程序作者
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
/// 开源许可证
pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");
/// 仓库地址
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// 家页面
pub const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let state = AppState::init(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            command::get_app_info,
            command::get_data,
            command::submit_excel_path,
            command::submit_data,
            command::submit_execute,
            command::get_execute,
            command::rescan,
            command::rename,
            command::recover,
            command::backup,
            command::clear_data,
        ])
        .run(tauri::generate_context!())
        .expect("启动 FormatName 失败");
}
