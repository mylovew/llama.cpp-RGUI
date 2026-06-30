use serde::{Deserialize, Serialize};

/// 从 GGUF 文件解析出的模型元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    pub path: String,
    pub file_size_bytes: u64,
    /// general.name
    #[serde(default)]
    pub name: Option<String>,
    /// general.architecture (llama, qwen2, etc.)
    pub architecture: String,
    /// general.file_type 数值
    #[serde(default)]
    pub file_type: Option<u32>,
    /// 量化类型名称 (如 Q4_K_M)
    #[serde(default)]
    pub file_type_name: Option<String>,
    /// <arch>.context_length
    #[serde(default)]
    pub context_length: Option<u32>,
    /// <arch>.embedding_length
    #[serde(default)]
    pub embedding_length: Option<u32>,
    /// <arch>.block_count (层数)
    #[serde(default)]
    pub block_count: Option<u32>,
    /// <arch>.attention.head_count
    #[serde(default)]
    pub head_count: Option<u32>,
    /// <arch>.attention.head_count_kv
    #[serde(default)]
    pub head_count_kv: Option<u32>,
    /// <arch>.attention.key_length
    #[serde(default)]
    pub key_length: Option<u32>,
    /// <arch>.attention.value_length
    #[serde(default)]
    pub value_length: Option<u32>,
    /// GGUF 版本
    pub gguf_version: u32,
}

/// 扫描结果中的模型项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub path: String,
    pub file_name: String,
    pub folder: String,
    pub file_size_bytes: u64,
    /// 解析成功时为 Some
    #[serde(default)]
    pub meta: Option<ModelMeta>,
    /// 解析失败时的错误信息
    #[serde(default)]
    pub parse_error: Option<String>,
}

/// 显存估算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramEstimate {
    /// 模型权重（≈ 文件大小）
    pub weights_bytes: u64,
    /// KV 缓存
    pub kv_cache_bytes: u64,
    /// 开销估算
    pub overhead_bytes: u64,
    /// 总计
    pub total_bytes: u64,
    /// GPU 显存占用（按 ngl 比例）
    pub gpu_offload_bytes: u64,
    /// 系统内存占用（非 offload 部分）
    pub system_ram_bytes: u64,
    /// 估算假设说明
    pub notes: Vec<String>,
}
