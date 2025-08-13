use serde::{Deserialize, Serialize};

/// 将 JSON 字符串解析为指定类型的结构体
pub fn parse_json<'a, T: Deserialize<'a>>(json_body: &'a str) -> Option<T> {
    let json_struct_res:serde_json::error::Result<T> = serde_json::from_str(json_body);
    match json_struct_res {
        Ok(data) => { Some(data) }
        Err(err) => {
            log::error!("json.2.struct error {:?}\n{:?}", json_body, err);
            None
        }
    }
}

/// 将 JSON 值解析为指定类型的结构体
pub fn parse_json_value<T: for<'a> Deserialize<'a>>(json_valve: &serde_json::Value) -> Option<T> {
    let json_struct_res:serde_json::error::Result<T> = serde_json::from_value(json_valve.to_owned());
    match json_struct_res {
        Ok(data) => { Some(data) }
        Err(err) => {
            log::error!("json-value.2.struct error {:?}\n{:?}", json_valve, err);
            None
        }
    }
}

/// 将 JSON 字符串解析为 JSON 值
pub fn parse_to_json_value(json_body: &str) -> serde_json::Value {
    serde_json::from_str(json_body).unwrap()
}

/// 将结构体序列化为 JSON 字符串
pub fn to_json_str<T: Serialize>(data: &T) -> String {
    let result = serde_json::to_string(data);
    match result {
        Ok(json_str) => { json_str }
        Err(err) => {
            log::error!("struct.2.json error {:?}", err);
            "{}".to_string()
        }
    }
}

/// 将一个结构体转换为另一个结构体
pub fn convert_struct<F: Serialize, T: for<'a> Deserialize<'a>>(from: F) -> Option<T> {
    let from_json = serde_json::to_string(&from).unwrap();
    let json_de_result: serde_json::error::Result<T> = serde_json::from_str(&from_json);
    match json_de_result {
        Ok(data) => { Some(data) }
        Err(err) => {
            log::error!("json parse to struct error {:?} {:?}", &from_json, err);
            None
        }
    }
}
