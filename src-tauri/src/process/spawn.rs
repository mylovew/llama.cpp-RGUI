use crate::config::schema::{FlashAttnMode, GpuLayers, Preset, SplitMode};

/// 根据预设构建 llama-server 命令行参数
pub fn build_args(model_path: &str, preset: &Preset) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // 模型路径（必填）
    args.push("-m".into());
    args.push(model_path.into());

    // 监听地址和端口
    args.push("--host".into());
    args.push(preset.host.clone());
    args.push("--port".into());
    args.push(preset.port.to_string());

    // 上下文长度
    if preset.ctx_size > 0 {
        args.push("-c".into());
        args.push(preset.ctx_size.to_string());
    }

    // GPU 卸载
    match &preset.n_gpu_layers {
        GpuLayers::All => {
            args.push("-ngl".into());
            args.push("99".into());
        }
        GpuLayers::Auto => {
            args.push("-ngl".into());
            args.push("auto".into());
        }
        GpuLayers::Count(n) => {
            args.push("-ngl".into());
            args.push(n.to_string());
        }
    }

    // 线程数
    if preset.threads > 0 {
        args.push("-t".into());
        args.push(preset.threads.to_string());
    }

    // Flash Attention
    match preset.flash_attn {
        FlashAttnMode::On => {
            args.push("-fa".into());
        }
        FlashAttnMode::Off => {
            args.push("--no-flash-attn".into());
        }
        FlashAttnMode::Auto => {
            args.push("-fa".into());
            args.push("auto".into());
        }
    }

    // 批处理大小
    if preset.batch_size > 0 {
        args.push("-b".into());
        args.push(preset.batch_size.to_string());
    }
    if preset.ubatch_size > 0 {
        args.push("-ub".into());
        args.push(preset.ubatch_size.to_string());
    }

    // 并行请求数
    if preset.parallel > 1 {
        args.push("-np".into());
        args.push(preset.parallel.to_string());
    }

    // KV 缓存类型
    args.push("--cache-type-k".into());
    args.push(format!("{:?}", preset.cache_type_k).to_lowercase());
    args.push("--cache-type-v".into());
    args.push(format!("{:?}", preset.cache_type_v).to_lowercase());

    // 连续批处理
    if preset.cont_batching {
        args.push("-cb".into());
    } else {
        args.push("--no-cont-batching".into());
    }

    // 模型别名
    if let Some(alias) = &preset.alias {
        if !alias.is_empty() {
            args.push("-a".into());
            args.push(alias.clone());
        }
    }

    // API 密钥
    if let Some(key) = &preset.api_key {
        if !key.is_empty() {
            args.push("--api-key".into());
            args.push(key.clone());
        }
    }

    // GPU 分割模式
    match preset.split_mode {
        SplitMode::None => {
            args.push("-sm".into());
            args.push("none".into());
        }
        SplitMode::Layer => {
            args.push("-sm".into());
            args.push("layer".into());
        }
        SplitMode::Row => {
            args.push("-sm".into());
            args.push("row".into());
        }
    }

    // 主 GPU
    if let Some(mg) = preset.main_gpu {
        args.push("--main-gpu".into());
        args.push(mg.to_string());
    }

    // Jinja
    if preset.jinja {
        args.push("--jinja".into());
    } else {
        args.push("--no-jinja".into());
    }

    // 自定义聊天模板
    if let Some(tpl) = &preset.chat_template {
        if !tpl.is_empty() {
            args.push("--chat-template".into());
            args.push(tpl.clone());
        }
    }

    // 追加的原始参数
    for arg in &preset.custom_args {
        args.push(arg.clone());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_basic() {
        let preset = Preset::default_preset();
        let args = build_args("/path/to/model.gguf", &preset);
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"/path/to/model.gguf".to_string()));
        assert!(args.contains(&"--host".to_string()));
        assert!(args.contains(&"--port".to_string()));
        assert!(args.contains(&"-ngl".to_string()));
    }
}
