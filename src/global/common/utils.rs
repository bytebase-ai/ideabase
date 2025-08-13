// 定义全局 Snowflake 变量
use lazy_static::lazy_static;
lazy_static! {
    static ref GLOBAL_SNOWFLAKE: std::sync::Mutex<rustflake::Snowflake> = std::sync::Mutex::new(rustflake::Snowflake::new(1420070400000, 1, 1));
}

/// 生成一个全局唯一的ID (基于Snowflake算法)
///
/// # 返回
/// 返回一个u64类型的唯一ID
pub fn get_next_id() -> i64 {
    GLOBAL_SNOWFLAKE.lock().unwrap().generate()
}


/// 将字节向量编码为Base64字符串
///
/// # 参数
/// * `bytes` - 需要编码的字节向量
///
/// # 返回
/// 返回Base64编码后的字符串
use base64::{Engine as _, engine::general_purpose};
pub fn base64_encode(bytes: Vec<u8>) -> String {
    general_purpose::STANDARD.encode(bytes)
}


/// 将serde_json::Map转换为std::collections::HashMap
///
/// # 参数
/// * `map` - 需要转换的serde_json::Map对象
///
/// # 返回
/// 返回一个新的HashMap，包含原Map中的所有键值对
pub fn serde_json_map_to_hashmap(map: &serde_json::Map<String, serde_json::Value>) -> std::collections::HashMap<String, serde_json::Value> {
    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// 生成安全的 API Key
///
/// # 参数
/// - `id`: 用户或实体的唯一标识符，类型为 `i64`
///
/// # 返回值
/// 返回一个格式为 `HEXID_UUID` 的大写字符串，其中 `HEXID` 是经过乱序处理的十六进制 ID，`UUID` 是去除连字符的 UUIDv7
pub fn generate_secure_api_key(id: i64) -> String {
    let hex_id = shuffle_hex_string(id);
    let uuid = uuid7::uuid7().to_string().replace("-", "");
    format!("{}_{}", hex_id, uuid).to_uppercase()
}

/// 对输入的 `i64` 值进行十六进制乱序处理
///
/// # 参数
/// - `id`: 需要乱序处理的 `i64` 值
///
/// # 返回值
/// 返回一个乱序后的十六进制字符串
pub fn shuffle_hex_string(id: i64) -> String {
    // 将i64转换为16进制字符串
    let hex_string = format!("{:x}", id);
    
    // 将字符串转换为字符向量
    let mut chars: Vec<char> = hex_string.chars().collect();
    
    // 使用简单的哈希算法进行乱序
    // 这里使用字符的ASCII值和位置进行简单的哈希计算
    let len = chars.len();
    for i in 0..len {
        let j = (i * 31 + chars[i] as usize) % len;
        chars.swap(i, j);
    }
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use crate::global::common::utils::generate_secure_api_key;

    #[test]
    fn test_generate_secure_api_key() {
        let api_key = generate_secure_api_key(11111i64);
        println!("{}", api_key);
    }
}