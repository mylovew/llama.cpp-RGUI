use crate::config;
use crate::error::{AppError, AppResult};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// 读取全局配置
#[tauri::command]
pub async fn load_config(app: AppHandle) -> AppResult<config::schema::AppConfig> {
    let state = app.state::<crate::AppState>();
    let config = state.config.read().await;
    Ok(config.clone())
}

/// 保存全局配置
#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    new_config: config::schema::AppConfig,
) -> AppResult<()> {
    // 持久化
    config::store::save(&app, &new_config)?;
    // 更新内存状态
    let state = app.state::<crate::AppState>();
    let mut config_guard = state.config.write().await;
    *config_guard = new_config;
    Ok(())
}

/// 文件选择对话框
#[tauri::command]
pub async fn pick_file(app: AppHandle, filter: Option<Vec<String>>) -> Option<String> {
    let mut builder = app.dialog().file();
    if let Some(exts) = filter {
        if !exts.is_empty() {
            let label = exts.join(", ");
            let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            builder = builder.add_filter(&label, &ext_refs);
        }
    }
    let path = builder.blocking_pick_file();
    path.map(|p| p.to_string())
}

/// 文件夹选择对话框
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    let path = app.dialog().file().blocking_pick_folder();
    path.map(|p| p.to_string())
}

/// 检测 llama-server 版本
#[tauri::command]
pub async fn detect_server_version(server_path: String) -> AppResult<String> {
    let mut cmd = tokio::process::Command::new(&server_path);
    cmd.arg("--version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::Process(format!("无法执行 {}: {}", server_path, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // 解析版本号，格式通常为 "version: 4400 (abcd1234)" 或 "ggml_cuda ...
    for line in combined.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version:") {
            // 提取 "version: XXXX" 中的数字
            let ver = trimmed
                .strip_prefix("version:")
                .unwrap_or("")
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !ver.is_empty() {
                return Ok(ver.to_string());
            }
        }
        if trimmed.contains("version") && trimmed.contains(':') {
            // 兜底：尝试从含 "version" 的行提取
            if let Some(pos) = trimmed.find(':') {
                let rest = trimmed[pos + 1..].trim();
                let ver = rest.split_whitespace().next().unwrap_or("");
                if !ver.is_empty() {
                    return Ok(ver.to_string());
                }
            }
        }
    }

    // 如果无法解析，返回原始输出的前 200 字符
    let snippet: String = combined.chars().take(200).collect();
    Ok(if snippet.trim().is_empty() {
        "unknown".to_string()
    } else {
        snippet.trim().to_string()
    })
}

/// GitHub 版本检查结果
#[derive(serde::Serialize)]
pub struct VersionCheckResult {
    /// 当前本地版本（原始字符串）
    pub current: Option<String>,
    /// 当前版本号（解析出的数字）
    pub current_number: Option<u32>,
    /// GitHub 最新 release tag（如 "b4500"）
    pub latest_tag: String,
    /// 最新版本号（解析出的数字）
    pub latest_number: Option<u32>,
    /// 是否有更新
    pub has_update: bool,
    /// release 页面 URL
    pub release_url: String,
    /// 发布时间
    pub published_at: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// 从字符串中提取首个连续数字序列
fn extract_number(s: &str) -> Option<u32> {
    let digits: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// 检查 GitHub 上 llama.cpp 的最新 release 版本，与本地版本对比
#[tauri::command]
pub async fn check_latest_version(current_version: Option<String>) -> VersionCheckResult {
    let current_number = current_version
        .as_ref()
        .and_then(|v| extract_number(v));

    let base = || VersionCheckResult {
        current: current_version.clone(),
        current_number,
        latest_tag: String::new(),
        latest_number: None,
        has_update: false,
        release_url: String::new(),
        published_at: None,
        error: None,
    };

    let client = match reqwest::Client::builder()
        .user_agent("llama-cpp-rgui")
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let mut r = base();
            r.error = Some(format!("HTTP 客户端构建失败: {}", e));
            return r;
        }
    };

    // 先尝试 releases/latest（自动跳过 pre-release / draft）
    let fetch_result = async {
        let resp = client
            .get("https://api.github.com/repos/ggml-org/llama.cpp/releases/latest")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            let v: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let tag = v["tag_name"].as_str().unwrap_or("").to_string();
            let url = v["html_url"].as_str().unwrap_or("").to_string();
            let published = v["published_at"].as_str().map(|s| s.to_string());
            if !tag.is_empty() {
                return Ok::<(String, String, Option<String>), String>((
                    tag,
                    url,
                    published,
                ));
            }
        }
        // 回退到 tags 接口
        let resp = client
            .get("https://api.github.com/repos/ggml-org/llama.cpp/tags?per_page=1")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let text = resp.text().await.map_err(|e| e.to_string())?;
        let arr: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let tag = arr[0]["name"].as_str().unwrap_or("").to_string();
        if tag.is_empty() {
            return Err("未找到任何版本标签".to_string());
        }
        let url = format!("https://github.com/ggml-org/llama.cpp/releases/tag/{}", tag);
        Ok((tag, url, None))
    };

    match fetch_result.await {
        Ok((tag, url, published)) => {
            let latest_number = extract_number(&tag);
            let has_update = matches!((current_number, latest_number), (Some(c), Some(l)) if l > c);
            VersionCheckResult {
                current: current_version,
                current_number,
                latest_tag: tag,
                latest_number,
                has_update,
                release_url: url,
                published_at: published,
                error: None,
            }
        }
        Err(e) => {
            let mut r = base();
            r.error = Some(e);
            r
        }
    }
}
