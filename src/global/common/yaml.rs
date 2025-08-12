use serde::{Deserialize, Serialize};
use figment::{Figment, providers::{Format, Yaml}};
use crate::global::common::json::json_to_json_value;

// 解析配置项目
pub fn load_env_yaml() -> GlobalEnv {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| String::from("dev"));
    let yml_dir = std::env::var("YML_DIR").unwrap_or_else(|_| String::from("yaml"));
    let main_conf = format!("{}/application.yaml", yml_dir);
    let active_conf = format!("{}/application-{}.yaml", yml_dir, profile);
    log::info!("load conf {}, {}", main_conf, active_conf);

    let global_env = Figment::new()
        .merge(Yaml::file(main_conf)).merge(Yaml::file(active_conf))
        .extract().expect("application yaml conf parse error");
    log::info!("core.env \n {}", serde_json::to_string_pretty(&global_env).unwrap());
    global_env
}

pub fn load_env_json(json_file: &str) -> serde_json::Value {
    let json_dir = std::env::var("YML_DIR").unwrap_or_else(|_| String::from("yaml"));
    let json_file_path = format!("{json_dir}/{json_file}");
    if !std::path::Path::new(&json_file_path).exists() {
        log::error!("json file {} not exists", json_file_path);
        serde_json::Value::Null
    } else {
        let json_data_str = std::fs::read_to_string(json_file_path).unwrap();
        json_to_json_value(&json_data_str)
    }
}


/// 配置变量
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalEnv {
    // 缓存配置
    pub cache: Cache,
    // jwt配置
    pub jwt: Jwt,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cache {
    // 本地缓存目录
    pub dir: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jwt {
    // jwt secret
    pub secret: String,
    // jwt expire hour
    pub expire_hour: u32,
}
