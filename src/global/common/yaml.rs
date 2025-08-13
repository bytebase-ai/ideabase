use serde::{Deserialize, Serialize};
use figment::{Figment, providers::{Format, Yaml}};

/// 从 YAML 配置文件加载全局环境配置
///
/// 该函数会读取两个配置文件：
/// 1. 主配置文件: `{YML_DIR}/application.yaml`
/// 2. 环境特定配置文件: `{YML_DIR}/application-{PROFILE}.yaml`
///
/// 环境变量说明：
/// - `PROFILE`: 运行环境 (默认: "dev")
/// - `YML_DIR`: 配置文件目录 (默认: "yaml")
///
/// 配置文件合并规则：环境特定配置会覆盖主配置中的同名字段
pub fn load_global_config() -> GlobalConfig {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| String::from("dev"));
    let yml_dir = std::env::var("YML_DIR").unwrap_or_else(|_| String::from("yaml"));
    let main_conf = format!("{}/application.yaml", yml_dir);
    let active_conf = format!("{}/application-{}.yaml", yml_dir, profile);
    log::info!("loading config files: {}, {}", main_conf, active_conf);

    let global_config = Figment::new()
        .merge(Yaml::file(main_conf))
        .merge(Yaml::file(active_conf))
        .extract()
        .expect("failed to parse application yaml configuration");

    log::info!("loaded global config: \n{}", serde_json::to_string_pretty(&global_config).unwrap());
    global_config
}

/// 全局配置结构体
///
/// 包含应用程序的所有配置项，通过 YAML 文件进行配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// 缓存相关配置
    pub cache: CacheConfig,
    /// JWT 认证相关配置
    pub jwt: JwtConfig,
}

/// 缓存配置结构体
///
/// 定义应用程序缓存相关的配置参数
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    /// 本地缓存目录路径
    pub dir: String,
}

/// JWT 配置结构体
///
/// 定义 JWT token 生成和验证所需的配置参数
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtConfig {
    /// JWT 签名密钥
    pub secret: String,
    /// JWT 过期时间（小时）
    pub expire_hour: u32,
}
