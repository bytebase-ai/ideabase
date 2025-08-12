use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

/// 使用 Argon2 算法对明文密码进行哈希处理，返回哈希后的字符串。
/// 
/// # 参数
/// - `passwd`: 需要加密的明文密码字符串引用
///
/// # 返回
/// - `Ok(String)`: 哈希后的密码字符串
/// - `Err(Box<dyn std::error::Error>)`: 发生错误时的错误信息
pub fn hash_passwd(passwd: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 生成一个随机盐值，增强哈希安全性
    let salt = SaltString::generate(&mut OsRng);
    // 创建默认配置的 Argon2 实例
    let argon2 = Argon2::default();
    // 对密码进行哈希处理，返回哈希字符串
    let hashed = argon2
        .hash_password(passwd.as_bytes(), &salt)?
        .to_string();
    Ok(hashed)
}

/// 校验明文密码与哈希密码是否匹配。
///
/// # 参数
/// - `passwd`: 明文密码字符串引用
/// - `hashed_passwd`: 哈希后的密码字符串引用
///
/// # 返回
/// - `true`: 密码匹配
/// - `false`: 密码不匹配或哈希解析失败
pub fn verify_passwd(passwd: &str, hashed_passwd: &str) -> bool {
    // 创建默认配置的 Argon2 实例
    let argon2 = Argon2::default();
    // 尝试解析哈希密码字符串
    if let Ok(parsed_hash) = PasswordHash::new(hashed_passwd) {
        // 验证明文密码与哈希是否匹配，匹配返回 true，否则返回 false
        argon2.verify_password(passwd.as_bytes(), &parsed_hash).is_ok()
    } else {
        // 哈希解析失败，直接返回 false
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::global::common::passwd::{hash_passwd, verify_passwd};

    #[test]
    fn test_hash_password() {
        let result = hash_passwd("Zkkk@1122");
        match result {
            Ok(hashed_passwd) => {
                println!("hashed_passwd: {}", hashed_passwd);
                let is_ok = verify_passwd("Zkkk@1122", &hashed_passwd);
                println!("is_ok: {is_ok}");
            }
            Err(err) => {
                println!("Err: {err}");
            }
        }
    }
}