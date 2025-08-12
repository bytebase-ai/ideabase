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

// 生成安全的API Key
pub fn do_generate_api_key(id: i64) -> String {
    let hex_id = hex_shuffle(id);
    let uuid = uuid7::uuid7().to_string().replace("-", "");
    format!("{}_{}", hex_id, uuid).to_uppercase()
}

pub fn hex_shuffle(id: i64) -> String {
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
    use crate::global::common::utils::do_generate_api_key;

    #[test]
    fn test_generate_secure_api_key() {
        let api_key = do_generate_api_key(11111i64);
        println!("{}", api_key);
    }
}