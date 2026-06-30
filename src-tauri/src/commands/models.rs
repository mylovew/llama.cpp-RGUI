use std::path::{Path, PathBuf};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::config::schema::Preset;
use crate::error::{AppError, AppResult};
use crate::gguf::{meta::*, parser, vram};

/// 判断目录/文件名是否与模型扫描无关。
/// 用于 WalkDir::filter_entry 剪枝：返回 true 的条目会被跳过（不 yield、不递归），
/// 避免误选宽目录（如整个磁盘）时扫描大量无关文件。根目录不受此影响。
fn is_irrelevant_dir(name: &std::ffi::OsStr) -> bool {
    let s = match name.to_str() {
        Some(s) => s,
        None => return false,
    };
    matches!(
        s,
        // 版本控制
        ".git" | ".svn" | ".hg" | ".bzr"
        // 依赖 / 缓存
        | "node_modules" | "__pycache__" | ".venv" | "venv" | ".mypy_cache"
        // 构建产物
        | "target" | "dist" | "build" | "out" | ".next" | ".nuxt"
        // Windows 系统目录
        | "$RECYCLE.BIN" | "System Volume Information" | "WindowsApps"
        // macOS / Linux 系统目录
        | ".Trash" | ".Trash-1000" | "Library" | "Applications"
        // 其他常见无关
        | ".DS_Store" | "Thumbs.db" | "desktop.ini"
    )
}

/// 扫描指定文件夹下的 .gguf 模型文件并解析元数据
#[tauri::command]
pub async fn scan_models(folders: Vec<String>) -> Vec<ModelInfo> {
    // 收集所有 .gguf 文件路径
    let mut gguf_files: Vec<(PathBuf, PathBuf)> = Vec::new(); // (file_path, folder)

    for folder in &folders {
        let folder_path = Path::new(folder);
        if !folder_path.exists() || !folder_path.is_dir() {
            continue;
        }
        // 不限制扫描深度：模型文件可能位于任意层级（如 models/org/model-name/variant/file.gguf）。
        // WalkDir 默认不跟随符号链接，无循环风险。为提升效率，遍历时跳过常见无关目录。
        for entry in WalkDir::new(folder_path)
            .into_iter()
            .filter_entry(|e| !is_irrelevant_dir(e.file_name()))
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq_ignore_ascii_case("gguf") {
                        gguf_files.push((entry.path().to_path_buf(), PathBuf::from(folder)));
                    }
                }
            }
        }
    }

    // 多文件夹可能存在路径重叠（如同时配置 D:\AI 和 D:\AI\models），
    // 同一个 .gguf 文件会被扫描多次。按文件绝对路径去重：
    // key 用小写化的字符串路径（兼容 Windows 大小写不敏感），
    // 保留 folder 路径更长（更具体）的那个，便于在 UI 上展示更准确的归属。
    let mut dedup: std::collections::HashMap<String, (PathBuf, PathBuf)> =
        std::collections::HashMap::new();
    for (path, folder) in gguf_files {
        let key = path.to_string_lossy().to_lowercase();
        dedup
            .entry(key)
            .and_modify(|(_, existing_folder)| {
                if folder.to_string_lossy().len() > existing_folder.to_string_lossy().len() {
                    *existing_folder = folder.clone();
                }
            })
            .or_insert((path, folder));
    }
    let gguf_files: Vec<(PathBuf, PathBuf)> = dedup.into_values().collect();

    // 并发解析
    let results: Vec<ModelInfo> = gguf_files
        .par_iter()
        .map(|(path, folder)| {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

            match parser::parse_gguf_file(path) {
                Ok(meta) => ModelInfo {
                    path: path.to_string_lossy().to_string(),
                    file_name,
                    folder: folder.to_string_lossy().to_string(),
                    file_size_bytes: file_size,
                    meta: Some(meta),
                    parse_error: None,
                },
                Err(e) => ModelInfo {
                    path: path.to_string_lossy().to_string(),
                    file_name,
                    folder: folder.to_string_lossy().to_string(),
                    file_size_bytes: file_size,
                    meta: None,
                    parse_error: Some(e.to_string()),
                },
            }
        })
        .collect();

    // 按文件名排序
    let mut sorted = results;
    sorted.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
    sorted
}

/// 解析单个 GGUF 文件
#[tauri::command]
pub async fn parse_gguf(path: String) -> AppResult<ModelMeta> {
    parser::parse_gguf_file(Path::new(&path))
}

/// 根据模型元数据和预设估算显存占用
#[tauri::command]
pub async fn estimate_vram(meta: ModelMeta, preset: Preset) -> VramEstimate {
    vram::estimate(&meta, &preset)
}
