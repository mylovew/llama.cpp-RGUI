use tokio::process::Child;

/// 跨平台停止子进程
pub async fn stop_child(child: &mut Child) {
    #[cfg(unix)]
    {
        stop_unix(child).await;
    }
    #[cfg(windows)]
    {
        stop_windows(child).await;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = child.kill().await;
    }
}

#[cfg(unix)]
async fn stop_unix(child: &mut Child) {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    if let Some(pid) = child.id() {
        // 向进程组发 SIGTERM（负 PID 表示进程组）
        let pgid = Pid::from_raw(-(pid as i32));
        let _ = kill(pgid, Signal::SIGTERM);

        // 等待 5 秒
        for _ in 0..50 {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
                Err(_) => return,
            }
        }

        // 超时强杀
        let _ = kill(pgid, Signal::SIGKILL);
    }
    let _ = child.wait().await;
}

#[cfg(windows)]
async fn stop_windows(child: &mut Child) {
    // Windows 上直接 kill 子进程
    // 注意：llama-server 可能有自己的子进程，但 kill 会终止整个进程树
    if let Some(pid) = child.id() {
        // 尝试用 taskkill /T 终止整个进程树
        let _ = tokio::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .await;
    }
    // 确保子进程已终止
    let _ = child.wait().await;
}
