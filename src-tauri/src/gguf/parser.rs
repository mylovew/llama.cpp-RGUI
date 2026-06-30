use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::gguf::meta::ModelMeta;
use crate::gguf::quant;

const GGUF_MAGIC: u32 = 0x46554747;

// GGUF metadata value types
const T_UINT8: u32 = 0;
const T_INT8: u32 = 1;
const T_UINT16: u32 = 2;
const T_INT16: u32 = 3;
const T_UINT32: u32 = 4;
const T_INT32: u32 = 5;
const T_FLOAT32: u32 = 6;
const T_BOOL: u32 = 7;
const T_STRING: u32 = 8;
const T_ARRAY: u32 = 9;
const T_UINT64: u32 = 10;
const T_INT64: u32 = 11;
const T_FLOAT64: u32 = 12;

/// 解析时保留的值（只关心 U32 和 String，其余跳过）
enum GgufValue {
    U32(u32),
    String(String),
    Skipped,
}

struct GgufReader<R: Read + Seek> {
    r: R,
}

impl<R: Read + Seek> GgufReader<R> {
    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut b = [0u8; 4];
        self.r.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut b = [0u8; 8];
        self.r.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    fn read_string(&mut self) -> std::io::Result<String> {
        let len = self.read_u64()? as usize;
        let mut b = vec![0u8; len];
        self.r.read_exact(&mut b)?;
        Ok(String::from_utf8_lossy(&b).to_string())
    }

    /// 读取一个 metadata 值，只保留 U32 和 String，其余跳过
    fn read_value(&mut self, vtype: u32) -> std::io::Result<GgufValue> {
        match vtype {
            T_UINT32 => Ok(GgufValue::U32(self.read_u32()?)),
            T_STRING => Ok(GgufValue::String(self.read_string()?)),
            T_UINT8 | T_INT8 | T_BOOL => {
                self.r.seek(SeekFrom::Current(1))?;
                Ok(GgufValue::Skipped)
            }
            T_UINT16 | T_INT16 => {
                self.r.seek(SeekFrom::Current(2))?;
                Ok(GgufValue::Skipped)
            }
            T_INT32 | T_FLOAT32 => {
                self.r.seek(SeekFrom::Current(4))?;
                Ok(GgufValue::Skipped)
            }
            T_UINT64 | T_INT64 | T_FLOAT64 => {
                self.r.seek(SeekFrom::Current(8))?;
                Ok(GgufValue::Skipped)
            }
            T_ARRAY => {
                let elem_type = self.read_u32()?;
                let count = self.read_u64()?;
                self.skip_array(elem_type, count)?;
                Ok(GgufValue::Skipped)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown GGUF value type: {}", vtype),
            )),
        }
    }

    /// 跳过数组元素
    fn skip_array(&mut self, elem_type: u32, count: u64) -> std::io::Result<()> {
        let elem_size: u64 = match elem_type {
            T_UINT8 | T_INT8 | T_BOOL => 1,
            T_UINT16 | T_INT16 => 2,
            T_UINT32 | T_INT32 | T_FLOAT32 => 4,
            T_UINT64 | T_INT64 | T_FLOAT64 => 8,
            T_STRING => {
                // 字符串数组需逐个跳过
                for _ in 0..count {
                    let _ = self.read_string()?;
                }
                return Ok(());
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown array element type: {}", elem_type),
                ))
            }
        };
        self.r.seek(SeekFrom::Current((count * elem_size) as i64))?;
        Ok(())
    }
}

/// 解析 GGUF 文件，提取关键元数据
pub fn parse_gguf_file(path: &Path) -> AppResult<ModelMeta> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut reader = GgufReader {
        r: BufReader::new(file),
    };

    let magic = reader.read_u32()?;
    if magic != GGUF_MAGIC {
        return Err(AppError::GgufParse(format!(
            "不是 GGUF 文件 (magic: 0x{:08X})",
            magic
        )));
    }

    let version = reader.read_u32()?;
    let _tensor_count = reader.read_u64()?;
    let kv_count = reader.read_u64()?;

    let mut kv: HashMap<String, GgufValue> = HashMap::new();

    for _ in 0..kv_count {
        let key = reader.read_string()?;
        let vtype = reader.read_u32()?;
        let value = reader.read_value(vtype)?;
        // 只保留标量，跳过的不存
        if !matches!(value, GgufValue::Skipped) {
            kv.insert(key, value);
        }
    }

    let architecture = match kv.get("general.architecture") {
        Some(GgufValue::String(s)) => s.clone(),
        _ => "unknown".to_string(),
    };

    let get_u32 = |k: &str| match kv.get(k) {
        Some(GgufValue::U32(v)) => Some(*v),
        _ => None,
    };
    let get_str = |k: &str| match kv.get(k) {
        Some(GgufValue::String(s)) => Some(s.clone()),
        _ => None,
    };

    let prefix = format!("{}.", architecture);
    let file_type = get_u32("general.file_type");

    Ok(ModelMeta {
        path: path.to_string_lossy().to_string(),
        file_size_bytes: file_size,
        name: get_str("general.name"),
        architecture,
        file_type,
        file_type_name: file_type.and_then(quant::file_type_to_name),
        context_length: get_u32(&format!("{}context_length", prefix)),
        embedding_length: get_u32(&format!("{}embedding_length", prefix)),
        block_count: get_u32(&format!("{}block_count", prefix)),
        head_count: get_u32(&format!("{}attention.head_count", prefix)),
        head_count_kv: get_u32(&format!("{}attention.head_count_kv", prefix)),
        key_length: get_u32(&format!("{}attention.key_length", prefix)),
        value_length: get_u32(&format!("{}attention.value_length", prefix)),
        gguf_version: version,
    })
}
