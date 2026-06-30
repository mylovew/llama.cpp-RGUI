use std::time::{Duration, Instant};
use tokio::net::TcpStream;

/// 检测端口是否已被占用（能连上说明被占用）
pub async fn is_port_in_use(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    tokio::net::TcpStream::connect(&addr).await.is_ok()
}

/// 轮询等待服务就绪
/// 先等 TCP 端口连通，再等 HTTP /health 返回 200
/// 返回 true 表示就绪，false 表示超时
pub async fn wait_for_ready(host: &str, port: u16, timeout_secs: u64) -> bool {
    let addr = format!("{}:{}", host, port);
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let interval = Duration::from_millis(500);

    // 阶段1：等待 TCP 端口监听
    while start.elapsed() < timeout {
        if TcpStream::connect(&addr).await.is_ok() {
            break;
        }
        tokio::time::sleep(interval).await;
    }
    if start.elapsed() >= timeout {
        return false;
    }

    // 阶段2：等待 HTTP /health 返回 200（模型加载完成）
    let health_url = format!("http://{}:{}/health", host, port);
    while start.elapsed() < timeout {
        match reqwest::get(&health_url).await {
            Ok(resp) if resp.status().is_success() => return true,
            _ => tokio::time::sleep(interval).await,
        }
    }
    false
}
