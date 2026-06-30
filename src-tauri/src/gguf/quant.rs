/// GGUF file_type 到量化名称的映射
/// 参考: llama.cpp ggml.h ggml_type 枚举
pub fn file_type_to_name(file_type: u32) -> Option<String> {
    Some(match file_type {
        0 => "F32".to_string(),
        1 => "F16".to_string(),
        2 => "Q4_0".to_string(),
        3 => "Q4_1".to_string(),
        6 => "Q5_0".to_string(),
        7 => "Q5_1".to_string(),
        8 => "Q8_0".to_string(),
        9 => "Q8_1".to_string(),
        10 => "Q2_K".to_string(),
        11 => "Q3_K_S".to_string(),
        12 => "Q3_K_M".to_string(),
        13 => "Q3_K_L".to_string(),
        14 => "Q4_K_S".to_string(),
        15 => "Q4_K_M".to_string(),
        16 => "Q5_K_S".to_string(),
        17 => "Q5_K_M".to_string(),
        18 => "Q6_K".to_string(),
        19 => "IQ2_XXS".to_string(),
        20 => "IQ2_XS".to_string(),
        21 => "Q2_K_S".to_string(),
        22 => "IQ3_XS".to_string(),
        23 => "IQ3_XXS".to_string(),
        24 => "IQ1_S".to_string(),
        25 => "IQ4_NL".to_string(),
        26 => "IQ3_S".to_string(),
        27 => "IQ3_M".to_string(),
        28 => "IQ2_S".to_string(),
        29 => "IQ2_M".to_string(),
        30 => "IQ1_M".to_string(),
        31 => "BF16".to_string(),
        32 => "Q4_0_4_4".to_string(),
        33 => "Q4_0_4_8".to_string(),
        34 => "Q4_0_8_8".to_string(),
        35 => "TQ1_0".to_string(),
        36 => "TQ2_0".to_string(),
        37 => "IQ4_XS".to_string(),
        _ => return None,
    })
}

/// 量化类型对应的每权重字节数（用于粗略估算，实际权重用文件大小更准）
pub fn file_type_bytes_per_weight(file_type: u32) -> f64 {
    match file_type {
        0 => 4.0,        // F32
        1 | 31 => 2.0,   // F16, BF16
        2 | 3 => 0.518,  // Q4_0, Q4_1
        6 | 7 => 0.625,  // Q5_0, Q5_1
        8 | 9 => 1.0,    // Q8_0, Q8_1
        10 | 21 => 0.25, // Q2_K, Q2_K_S
        11 => 0.25,      // Q3_K_S
        12 => 0.313,     // Q3_K_M
        13 => 0.375,     // Q3_K_L
        14 => 0.5,       // Q4_K_S
        15 => 0.563,     // Q4_K_M
        16 => 0.625,     // Q5_K_S
        17 => 0.688,     // Q5_K_M
        18 => 0.75,      // Q6_K
        19..=20 => 0.188, // IQ2_XXS, IQ2_XS
        22..=23 => 0.313, // IQ3_XS, IQ3_XXS
        24 => 0.188,     // IQ1_S
        25 | 37 => 0.5,  // IQ4_NL, IQ4_XS
        26..=27 => 0.313, // IQ3_S, IQ3_M
        28..=29 => 0.25, // IQ2_S, IQ2_M
        30 => 0.188,     // IQ1_M
        35..=36 => 0.125, // TQ1_0, TQ2_0
        _ => 0.5,        // 默认保守估计
    }
}
