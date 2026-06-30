mod commands;
mod config;
mod error;
mod gguf;
mod process;

use tauri::Manager;
use tokio::sync::{Mutex, RwLock};

use crate::config::AppConfig;
use crate::process::manager::{ServerRuntime, ServerState};

/// 应用全局状态
pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub server: Mutex<Option<ServerRuntime>>,
    pub logs: Mutex<Vec<LogEntry>>,
}

/// 单条日志
#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub ts: i64,
    pub level: String,
    pub line: String,
}

/// 服务运行状态（前端可查询）
#[derive(Clone, serde::Serialize)]
pub struct ServerStatus {
    pub state: ServerState,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub model: Option<String>,
    pub started_at: Option<i64>,
    pub message: Option<String>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 初始化配置目录与默认配置
            let config = config::store::load_or_init(app.handle())?;
            let state = AppState {
                config: RwLock::new(config),
                server: Mutex::new(None),
                logs: Mutex::new(Vec::new()),
            };
            app.manage(state);

            // 应用退出时清理子进程
            let handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        let handle = handle.clone();
                        // 在退出时尝试停止服务
                        tauri::async_runtime::block_on(async move {
                            let _ = process::manager::stop_server_internal(&handle).await;
                        });
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 配置
            commands::settings::load_config,
            commands::settings::save_config,
            // 文件选择
            commands::settings::pick_file,
            commands::settings::pick_folder,
            // 版本检测
            commands::settings::detect_server_version,
            commands::settings::check_latest_version,
            // 模型扫描
            commands::models::scan_models,
            commands::models::parse_gguf,
            commands::models::estimate_vram,
            // 进程管理
            commands::server::check_port,
            commands::server::start_server,
            commands::server::stop_server,
            commands::server::server_status,
            // 聊天窗口
            commands::webview::open_chat_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
