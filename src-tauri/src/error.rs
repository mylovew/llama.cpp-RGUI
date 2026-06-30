use serde::Serialize;

/// 统一错误类型，可序列化传递给前端
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Tauri 错误: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("GGUF 解析错误: {0}")]
    GgufParse(String),

    #[error("进程错误: {0}")]
    Process(String),

    #[error("端口 {port} 已被占用")]
    PortInUse { port: u16 },

    #[error("服务未运行")]
    NotRunning,

    #[error("服务已在运行")]
    AlreadyRunning,

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type AppResult<T> = Result<T, AppError>;
