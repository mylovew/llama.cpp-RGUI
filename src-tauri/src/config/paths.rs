use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 获取配置目录路径
pub fn config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| {
            dirs::config_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("com.llamacpp.rgui")
        })
}

/// 配置文件完整路径
pub fn config_file(app: &AppHandle) -> PathBuf {
    config_dir(app).join("config.json")
}

/// 确保配置目录存在
pub fn ensure_config_dir(app: &AppHandle) -> std::io::Result<()> {
    let dir = config_dir(app);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}
