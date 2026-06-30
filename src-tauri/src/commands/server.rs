use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};
use tokio::process::Command;

use crate::config::schema::Preset;
use crate::error::{AppError, AppResult};
use crate::process::health;
use crate::process::log_pipe;
use crate::process::manager::{emit_status, ServerRuntime, ServerState};
use crate::process::spawn as spawn_mod;
use crate::AppState;

/// 检查端口是否被占用
#[tauri::command]
pub async fn check_port(host: String, port: u16) -> bool {
    health::is_port_in_use(&host, port).await
}

/// 启动 llama-server
#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    model_path: String,
    preset: Preset,
    server_path: String,
) -> AppResult<serde_json::Value> {
    // 校验路径
    if !Path::new(&server_path).exists() {
        return Err(AppError::Config(format!(
            "llama-server 路径不存在: {}",
            server_path
        )));
    }
    if !Path::new(&model_path).exists() {
        return Err(AppError::Config(format!("模型文件不存在: {}", model_path)));
    }

    // 检查是否已有服务运行
    {
        let state = app.state::<AppState>();
        let guard = state.server.lock().await;
        if guard.is_some() {
            return Err(AppError::AlreadyRunning);
        }
    }

    // 检查端口是否被占用
    if health::is_port_in_use(&preset.host, preset.port).await {
        return Err(AppError::PortInUse { port: preset.port });
    }

    // 构建参数
    let args = spawn_mod::build_args(&model_path, &preset);
    let host = preset.host.clone();
    let port = preset.port;

    // 创建命令
    let mut cmd = Command::new(&server_path);
    cmd.args(&args);

    // 跨平台进程组设置
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                let _ = nix::unistd::setsid();
                Ok(())
            });
        }
    }

    // 设置环境变量
    for (k, v) in &preset.extra_env {
        cmd.env(k, v);
    }

    // 启用 stdout/stderr 捕获
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // emit Starting
    let _ = emit_status(&app, ServerState::Starting, Some("正在启动...".into()));

    // spawn
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Process(format!("启动 llama-server 失败: {}", e)))?;

    let pid = child.id().unwrap_or(0);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // 存入状态
    let runtime = ServerRuntime {
        child,
        pid,
        port,
        host: host.clone(),
        model: model_path.clone(),
        started_at: ServerRuntime::now_ts(),
    };

    {
        let state = app.state::<AppState>();
        let mut guard = state.server.lock().await;
        *guard = Some(runtime);
    }

    // 启动日志管道
    let app_for_logs = app.clone();
    tokio::spawn(async move {
        log_pipe::pipe_logs(app_for_logs, stdout, stderr).await;
    });

    // 启动就绪检测
    let app_for_ready = app.clone();
    let model_name = preset
        .alias
        .clone()
        .unwrap_or_else(|| {
            Path::new(&model_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "model".to_string())
        });
    let chat_host = host.clone();
    let chat_port = port;
    tokio::spawn(async move {
        let ready = health::wait_for_ready(&chat_host, chat_port, 180).await;
        if ready {
            // /health 返回 200 后再额外等 1 秒，确保 Web UI 静态资源已就绪
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let _ = app_for_ready.emit(
                "server://ready",
                &serde_json::json!({ "port": chat_port, "host": chat_host }),
            );
            // 发送 Running 状态
            let status = crate::ServerStatus {
                state: ServerState::Running,
                pid: Some(pid),
                port: Some(chat_port),
                host: Some(chat_host.clone()),
                model: Some(model_name.clone()),
                started_at: Some(ServerRuntime::now_ts()),
                message: Some("服务已就绪".into()),
            };
            let _ = app_for_ready.emit("server://status", &status);
        } else {
            let _ = emit_status(
                &app_for_ready,
                ServerState::Crashed,
                Some("服务启动超时（120s），请检查日志".into()),
            );
        }
    });

    // 启动进程监控（检测意外退出）
    let app_for_monitor = app.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let state = app_for_monitor.state::<AppState>();
            let mut guard = state.server.lock().await;
            if let Some(runtime) = guard.as_mut() {
                match runtime.child.try_wait() {
                    Ok(Some(_)) => {
                        // 进程已退出
                        guard.take();
                        drop(guard);
                        let _ = emit_status(
                            &app_for_monitor,
                            ServerState::Crashed,
                            Some("进程已退出".into()),
                        );
                        break;
                    }
                    Ok(None) => continue,
                    Err(_) => {
                        guard.take();
                        drop(guard);
                        let _ = emit_status(
                            &app_for_monitor,
                            ServerState::Crashed,
                            Some("进程状态检查失败".into()),
                        );
                        break;
                    }
                }
            } else {
                // server 已被清除（stop_server 调用）
                break;
            }
        }
    });

    Ok(serde_json::json!({
        "pid": pid,
        "port": port,
        "host": host,
    }))
}

/// 停止运行中的服务
#[tauri::command]
pub async fn stop_server(app: AppHandle) -> AppResult<()> {
    let _ = emit_status(&app, ServerState::Stopping, Some("正在停止...".into()));

    let state = app.state::<AppState>();
    let mut guard = state.server.lock().await;
    if let Some(mut runtime) = guard.take() {
        crate::process::shutdown::stop_child(&mut runtime.child).await;
        let _ = emit_status(&app, ServerState::Stopped, Some("已停止".into()));
        Ok(())
    } else {
        Err(AppError::NotRunning)
    }
}

/// 查询服务状态
#[tauri::command]
pub async fn server_status(app: AppHandle) -> crate::ServerStatus {
    let state = app.state::<AppState>();
    let guard = state.server.lock().await;
    match guard.as_ref() {
        Some(rt) => crate::ServerStatus {
            state: ServerState::Running,
            pid: Some(rt.pid),
            port: Some(rt.port),
            host: Some(rt.host.clone()),
            model: Some(rt.model.clone()),
            started_at: Some(rt.started_at),
            message: None,
        },
        None => crate::ServerStatus {
            state: ServerState::Stopped,
            pid: None,
            port: None,
            host: None,
            model: None,
            started_at: None,
            message: None,
        },
    }
}
