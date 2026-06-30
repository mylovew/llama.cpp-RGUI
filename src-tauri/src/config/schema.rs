use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 全局应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// llama-server 可执行文件路径
    #[serde(default)]
    pub server_path: Option<String>,
    /// 上次检测到的版本号
    #[serde(default)]
    pub server_version: Option<String>,
    /// 模型扫描文件夹列表
    #[serde(default)]
    pub model_folders: Vec<String>,
    /// 默认模型路径
    #[serde(default)]
    pub default_model_path: Option<String>,
    /// 默认预设 id
    #[serde(default)]
    pub default_preset_id: Option<String>,
    /// 预设列表
    #[serde(default)]
    pub presets: Vec<Preset>,
    /// 上次使用的预设 id
    #[serde(default)]
    pub last_used_preset_id: Option<String>,
    /// 上次使用的模型路径
    #[serde(default)]
    pub last_used_model_path: Option<String>,
    /// 主题模式
    #[serde(default)]
    pub theme: ThemeMode,
    /// 窗口状态
    #[serde(default)]
    pub window: WindowState,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_path: None,
            server_version: None,
            model_folders: Vec::new(),
            default_model_path: None,
            default_preset_id: None,
            presets: vec![Preset::default_preset()],
            last_used_preset_id: None,
            last_used_model_path: None,
            theme: ThemeMode::System,
            window: WindowState::default(),
        }
    }
}

/// 启动参数预设
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    // 基础参数
    /// 最大上下文长度 (-c)，0 表示使用模型默认值
    #[serde(default = "default_ctx")]
    pub ctx_size: u32,
    /// GPU 卸载层数 (-ngl)
    #[serde(default)]
    pub n_gpu_layers: GpuLayers,
    /// 线程数 (-t)，-1 表示自动
    #[serde(default = "default_threads")]
    pub threads: i32,
    /// 监听地址 (--host)
    #[serde(default = "default_host")]
    pub host: String,
    /// 监听端口 (-p)
    #[serde(default = "default_port")]
    pub port: u16,
    /// 模型别名 (-a)
    #[serde(default)]
    pub alias: Option<String>,
    // 性能参数
    /// Flash Attention (-fa)
    #[serde(default)]
    pub flash_attn: FlashAttnMode,
    /// 批处理大小 (-b)
    #[serde(default = "default_batch")]
    pub batch_size: u32,
    /// 微批处理大小 (-ub)
    #[serde(default = "default_ubatch")]
    pub ubatch_size: u32,
    /// 并行请求数 (-np)
    #[serde(default = "default_parallel")]
    pub parallel: i32,
    // 缓存参数
    /// K 缓存类型 (--cache-type-k)
    #[serde(default = "default_cache_type")]
    pub cache_type_k: CacheType,
    /// V 缓存类型 (--cache-type-v)
    #[serde(default = "default_cache_type")]
    pub cache_type_v: CacheType,
    /// 连续批处理 (-cb)
    #[serde(default = "default_true")]
    pub cont_batching: bool,
    // 安全
    /// API 密钥 (--api-key)
    #[serde(default)]
    pub api_key: Option<String>,
    // 高级
    /// GPU 分割模式 (--split-mode)
    #[serde(default)]
    pub split_mode: SplitMode,
    /// 主 GPU (--main-gpu)
    #[serde(default)]
    pub main_gpu: Option<u32>,
    /// 使用 Jinja 模板 (--jinja)
    #[serde(default = "default_true")]
    pub jinja: bool,
    /// 自定义聊天模板 (--chat-template)
    #[serde(default)]
    pub chat_template: Option<String>,
    /// 追加的原始参数
    #[serde(default)]
    pub custom_args: Vec<String>,
    /// 额外环境变量
    #[serde(default)]
    pub extra_env: HashMap<String, String>,
}

impl Preset {
    pub fn default_preset() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "默认".to_string(),
            ctx_size: default_ctx(),
            n_gpu_layers: GpuLayers::Auto,
            threads: default_threads(),
            host: default_host(),
            port: default_port(),
            alias: None,
            flash_attn: FlashAttnMode::Auto,
            batch_size: default_batch(),
            ubatch_size: default_ubatch(),
            parallel: default_parallel(),
            cache_type_k: default_cache_type(),
            cache_type_v: default_cache_type(),
            cont_batching: true,
            api_key: None,
            split_mode: SplitMode::Layer,
            main_gpu: None,
            jinja: true,
            chat_template: None,
            custom_args: Vec::new(),
            extra_env: HashMap::new(),
        }
    }
}

/// GPU 卸载层数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GpuLayers {
    Count(u32),
    Auto,
    All,
}

impl Default for GpuLayers {
    fn default() -> Self {
        GpuLayers::Auto
    }
}

/// Flash Attention 模式
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FlashAttnMode {
    On,
    #[default]
    Off,
    Auto,
}

/// KV 缓存类型
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CacheType {
    F32,
    #[default]
    F16,
    BF16,
    Q8_0,
    Q4_0,
    Q4_1,
    Q5_0,
    Q5_1,
    IQ4_NL,
}

impl CacheType {
    /// 每个元素占用的字节数（用于显存估算）
    pub fn bytes_per_element(&self) -> f64 {
        match self {
            CacheType::F32 => 4.0,
            CacheType::F16 => 2.0,
            CacheType::BF16 => 2.0,
            CacheType::Q8_0 => 1.0,
            CacheType::Q4_0 => 0.518,
            CacheType::Q4_1 => 0.563,
            CacheType::Q5_0 => 0.625,
            CacheType::Q5_1 => 0.688,
            CacheType::IQ4_NL => 0.5,
        }
    }
}

/// GPU 分割模式
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SplitMode {
    None,
    #[default]
    Layer,
    Row,
}

/// 主题模式
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

/// 窗口状态（记忆窗口大小和位置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    #[serde(default = "default_win_width")]
    pub width: u32,
    #[serde(default = "default_win_height")]
    pub height: u32,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default = "default_true")]
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: default_win_width(),
            height: default_win_height(),
            x: None,
            y: None,
            maximized: false,
        }
    }
}

// 默认值函数
fn default_ctx() -> u32 { 0 }
fn default_threads() -> i32 { -1 }
fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8080 }
fn default_batch() -> u32 { 2048 }
fn default_ubatch() -> u32 { 512 }
fn default_parallel() -> i32 { 1 }
fn default_cache_type() -> CacheType { CacheType::F16 }
fn default_true() -> bool { true }
fn default_win_width() -> u32 { 1000 }
fn default_win_height() -> u32 { 680 }
