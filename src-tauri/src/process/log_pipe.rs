use std::collections::VecDeque;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::mpsc;

use crate::AppState;
use tauri::{AppHandle, Emitter, Manager};

const MAX_LOGS: usize = 10000;

/// 解析日志级别（llama.cpp 格式：I/W/E/D 前缀 或 [I]/[W]/[E] 标记）
fn parse_level(line: &str) -> &'static str {
    // llama.cpp 格式: I0930 12:00:00 ... 或 [I] ...
    let trimmed = line.trim_start();
    if trimmed.starts_with('E') || trimmed.starts_with("[E]") || trimmed.starts_with("ERROR") {
        "error"
    } else if trimmed.starts_with('W') || trimmed.starts_with("[W]") || trimmed.starts_with("WARN") {
        "warn"
    } else if trimmed.starts_with('D') || trimmed.starts_with("[D]") || trimmed.starts_with("DEBUG") {
        "debug"
    } else {
        "info"
    }
}

/// 异步管道：读取 stdout + stderr，推送到前端 + 存入环形缓冲
pub async fn pipe_logs(
    app: AppHandle,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
) {
    let (tx, mut rx) = mpsc::channel::<(String, String)>(256);

    if let Some(stdout) = stdout {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(("stdout".into(), line)).await.is_err() {
                    break;
                }
            }
        });
    }

    if let Some(stderr) = stderr {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(("stderr".into(), line)).await.is_err() {
                    break;
                }
            }
        });
    }

    drop(tx);

    while let Some((_source, line)) = rx.recv().await {
        let level = parse_level(&line);
        let entry = crate::LogEntry {
            ts: chrono::Local::now().timestamp(),
            level: level.into(),
            line: line.clone(),
        };

        // 存入环形缓冲
        if let Some(state) = app.try_state::<AppState>() {
            let mut logs = state.logs.lock().await; // 使用 std::sync::Mutex 则用 .lock().unwrap()
            // 注意: AppState.logs 是 tokio::sync::Mutex
            if logs.len() >= MAX_LOGS {
                logs.remove(0);
            }
            logs.push(entry.clone());
        }

        // emit 到前端
        let _ = app.emit("server://log", &entry);
    }
}
