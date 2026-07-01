use std::path::{Path, PathBuf};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::config::schema::Preset;
use crate::error::{AppError, AppResult};
use crate::gguf::{meta::*, parser, vram};

/// 判断文件名是否属于视觉辅助模型文件（mmproj），不应列入主模型列表
fn is_mmproj_file(name: &str) -> bool {
    name.to_lowercase().contains("mmproj")
}

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
                        // 跳过视觉辅助模型文件，不列入主模型列表
                        if is_mmproj_file(&entry.file_name().to_string_lossy()) {
                            continue;
                        }
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

/// 在模型文件所在目录自动查找 mmproj（视觉辅助模型）文件
/// 仅当恰好找到 1 个候选时返回，0 个或多于 1 个均返回 None
pub fn find_mmproj_near_model(model_path: &str) -> Option<String> {
    let dir = Path::new(model_path).parent()?;
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_lower = name.to_string_lossy().to_lowercase();
            if name_lower.ends_with(".gguf") && name_lower.contains("mmproj") {
                candidates.push(entry.path());
            }
        }
    }
    match candidates.len() {
        1 => Some(candidates[0].to_string_lossy().to_string()),
        _ => None,
    }
}

/// 查找模型旁边的 mmproj 文件（Tauri command，供前端预览）
#[tauri::command]
pub fn find_mmproj(model_path: String) -> Option<String> {
    find_mmproj_near_model(&model_path)
}
