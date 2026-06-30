use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

/// 服务运行状态
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
}

/// 运行中的服务实例信息
pub struct ServerRuntime {
    pub child: tokio::process::Child,
    pub pid: u32,
    pub port: u16,
    pub host: String,
    pub model: String,
    pub started_at: i64,
}

impl ServerRuntime {
    pub fn now_ts() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

/// 内部停止函数（应用退出时调用）
pub async fn stop_server_internal(app: &tauri::AppHandle) -> Result<(), crate::error::AppError> {
    let state = app.state::<crate::AppState>();
    let mut server_guard = state.server.lock().await;
    if let Some(mut runtime) = server_guard.take() {
        crate::process::shutdown::stop_child(&mut runtime.child).await;
    }
    let _ = emit_status(app, ServerState::Stopped, None);
    Ok(())
}

/// 发送状态变更事件到前端
pub fn emit_status(
    app: &tauri::AppHandle,
    state: ServerState,
    message: Option<String>,
) -> Result<(), tauri::Error> {
    use tauri::Emitter;
    let status = crate::ServerStatus {
        state,
        pid: None,
        port: None,
        host: None,
        model: None,
        started_at: None,
        message,
    };
    app.emit("server://status", &status)
}
