use crate::config::schema::{GpuLayers, Preset};
use crate::gguf::meta::{ModelMeta, VramEstimate};

const MB: u64 = 1024 * 1024;

/// 根据模型元数据和启动参数预设估算显存占用
pub fn estimate(meta: &ModelMeta, preset: &Preset) -> VramEstimate {
    let mut notes: Vec<String> = Vec::new();

    // 模型权重 ≈ 文件大小
    let weights = meta.file_size_bytes;

    // 层数
    let n_layers = meta.block_count.unwrap_or(32) as u64;
    // 上下文长度
    let n_ctx = if preset.ctx_size > 0 {
        preset.ctx_size as u64
    } else {
        meta.context_length.unwrap_or(4096) as u64
    };

    // head_dim: 优先 key_length，否则 embedding_length / head_count
    let head_dim = if let Some(kl) = meta.key_length {
        kl as u64
    } else if let (Some(emb), Some(hc)) = (meta.embedding_length, meta.head_count) {
        if hc > 0 {
            (emb / hc) as u64
        } else {
            128
        }
    } else {
        notes.push("无法读取 head_dim，按 128 估算".to_string());
        128
    };

    // KV 头数（GQA 后）
    let n_head_kv = meta
        .head_count_kv
        .unwrap_or_else(|| meta.head_count.unwrap_or(32)) as u64;

    // KV 缓存 = 2 × n_layers × n_ctx × n_head_kv × head_dim × dtype_bytes × parallel
    let dtype_k = preset.cache_type_k.bytes_per_element();
    let dtype_v = preset.cache_type_v.bytes_per_element();
    let dtype_max = dtype_k.max(dtype_v);
    let parallel = preset.parallel.max(1) as u64;

    let kv_cache = 2 * n_layers * n_ctx * n_head_kv * head_dim * (dtype_max as u64) * parallel;

    // 开销: max(256MiB, 5% × weights)
    let overhead = std::cmp::max(256 * MB, weights / 20);

    let total = weights + kv_cache + overhead;

    // GPU offload 比例
    let offload_ratio: f64 = match &preset.n_gpu_layers {
        GpuLayers::All => 1.0,
        GpuLayers::Auto => {
            notes.push("GPU 卸载设为 Auto，按全部卸载估算，实际取决于显存".to_string());
            1.0
        }
        GpuLayers::Count(ngl) => {
            if n_layers > 0 {
                (*ngl as f64 / n_layers as f64).min(1.0)
            } else {
                1.0
            }
        }
    };

    let gpu_offload = (offload_ratio * weights as f64) as u64 + kv_cache + overhead;
    let system_ram = ((1.0 - offload_ratio) * weights as f64) as u64;

    notes.push(format!(
        "上下文 {} 层 {} KV头 {} head_dim {}",
        n_ctx, n_layers, n_head_kv, head_dim
    ));
    if parallel > 1 {
        notes.push(format!("并行 {} (KV 缓存已乘以并行数)", parallel));
    }
    notes.push("估算误差约 ±10-15%".to_string());

    VramEstimate {
        weights_bytes: weights,
        kv_cache_bytes: kv_cache,
        overhead_bytes: overhead,
        total_bytes: total,
        gpu_offload_bytes: gpu_offload,
        system_ram_bytes: system_ram,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::*;

    #[test]
    fn test_estimate_basic() {
        let meta = ModelMeta {
            path: "/tmp/test.gguf".into(),
            file_size_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            name: Some("test".into()),
            architecture: "llama".into(),
            file_type: Some(15), // Q4_K_M
            file_type_name: Some("Q4_K_M".into()),
            context_length: Some(4096),
            embedding_length: Some(4096),
            block_count: Some(32),
            head_count: Some(32),
            head_count_kv: Some(8),
            key_length: Some(128),
            value_length: Some(128),
            gguf_version: 3,
        };
        let preset = Preset::default_preset();
        let est = estimate(&meta, &preset);

        // KV cache = (K+V) = 2 * 32 * 4096 * 8 * 128 * 2(F16) = 536870912 = 512MB
        assert_eq!(est.kv_cache_bytes, 512 * MB);
        assert!(est.total_bytes > 4 * 1024 * 1024 * 1024);
    }
}
