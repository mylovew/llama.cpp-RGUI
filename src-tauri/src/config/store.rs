use crate::config::paths;
use crate::config::schema::AppConfig;
use crate::error::AppResult;
use std::path::Path;
use tauri::AppHandle;

/// 加载配置，若不存在则创建默认配置并写入
pub fn load_or_init(app: &AppHandle) -> AppResult<AppConfig> {
    paths::ensure_config_dir(app)?;
    let path = paths::config_file(app);
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let config: AppConfig = serde_json::from_str(&content).unwrap_or_default();
        Ok(config)
    } else {
        let config = AppConfig::default();
        save_to_file(&path, &config)?;
        Ok(config)
    }
}

/// 保存配置（原子写入：先写临时文件再 rename）
pub fn save(app: &AppHandle, config: &AppConfig) -> AppResult<()> {
    paths::ensure_config_dir(app)?;
    let path = paths::config_file(app);
    save_to_file(&path, config)
}

fn save_to_file(path: &Path, config: &AppConfig) -> AppResult<()> {
    let json = serde_json::to_string_pretty(config)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
