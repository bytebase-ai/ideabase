use std::collections::HashMap;
use http::status::StatusCode;
use crate::global::common::rpc::RpcResult;
use crate::global::db::core::{is_table_exists, DBConn};

/// 处理删除数据的请求
/// 
/// # 参数
/// * `body_map` - 包含删除请求的数据映射，key为表名，value为删除条件
///
pub async fn handle_delete(db: &DBConn, body_map: HashMap<String, serde_json::Value>) -> RpcResult::<HashMap<String, serde_json::Value>> {
    let mut rpc_result = RpcResult::<HashMap<String, serde_json::Value>>{ code: StatusCode::OK, msg: None, payload: None };

    // 初始化结果映射，用于存储每个表的处理结果
    let mut result_payload = HashMap::<String, serde_json::Value>::new();
    // 遍历请求体中的每个表及其参数 -> todo 事务保证数据操作结果一致性
    for (table_key, param) in body_map {
        // 检查参数是否为JSON对象格式
        match param.as_object() {
            Some(param_map) => {
                // 解析 schema & table
                let schema: &str;
                let table: &str;
                let schema_table_vec = table_key.split('.').collect::<Vec<&str>>();
                if schema_table_vec.len() == 2 {
                    schema = schema_table_vec[0];
                    table = schema_table_vec[1];
                } else {
                    rpc_result.code = StatusCode::BAD_REQUEST;
                    result_payload.insert((&table_key).to_string(), serde_json::json!(format!("{}'s schema empty", &table_key)));
                    continue;
                }
                // 检查表是否存在，不存在则记录错误
                if !is_table_exists(&schema, &table) {
                    rpc_result.code = StatusCode::BAD_REQUEST;
                    result_payload.insert((&table_key).to_string(), serde_json::json!(format!("table {} not exists", &table_key)));
                    continue;
                }

                // 删除操作
                match do_delete(db, &schema, &table, param_map).await {
                    Ok(n) => { // 删除成功，记录影响行数
                        result_payload.insert((&table_key).to_string(), serde_json::json!(n));
                    },
                    Err(err) => { // 删除失败，记录错误信息
                        result_payload.insert((&table_key).to_string(), serde_json::json!(err));
                    }
                }
            },
            None => { // 参数格式错误，直接返回错误响应
                rpc_result.code = StatusCode::BAD_REQUEST;
                rpc_result.msg = Some("parameter format error".to_string());
            }
        }
    }
    if !result_payload.is_empty() {
        rpc_result.payload = Some(result_payload);
    }
    rpc_result
}

/// 执行数据删除操作
/// 
/// # 参数
/// * `db` - 数据库连接实例
/// * `table` - 要操作的表名
/// * `kvs` - 删除条件，支持两种格式：
///   * `{"id": number}` - 删除单条记录
///   * `{"id{}": [number]}` - 批量删除多条记录
/// 
/// # 返回值
/// * `Ok(u64)` - 成功时返回受影响的行数
/// * `Err(String)` - 失败时返回错误信息
/// 
/// # 错误情况
/// * id 值类型不是数字
/// * id{} 值类型不是数组
/// * 没有提供 id 或 id{} 字段
async fn do_delete(db: &DBConn, schema: &str, table: &str, kvs: &serde_json::Map<String, serde_json::Value>) -> Result<u64, String> {
    if let Some(id_value) = kvs.get("id") {
        // 处理单个 ID 删除
        if !id_value.is_number() {
            log::warn!("delete.do_delete id: {:?}", id_value);
            return Err(format!("'id' type is not num, key: {}, kvs: {:?}", table, kvs));
        }
        let sql = format!("delete from {}.{} where id={}", schema, table, id_value.to_string());
        execute_delete(db, &sql).await
    } else if let Some(id_array) = kvs.get("id{}") {
        // 处理批量 ID 删除
        if !id_array.is_array() {
            log::warn!("wrong id array: {:?}", id_array);
            return Err(format!("'id{{}}' type is not num array, key: {}, kvs: {:?}", table, kvs));
        }
        let id_arr = id_array.as_array().unwrap();
        let placeholders = (0..id_arr.len()).map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("delete from {}.{} where id in({})", schema, table, placeholders);
        execute_delete(db, &sql).await
    } else {
        // 没有提供有效的 ID
        Err(format!("data delete must have field 'id' or 'id{{}}', key: {}, kvs: {:?}", table, kvs))
    }
}

/// 执行实际的删除 SQL 操作
/// 
/// # 参数
/// * `db` - 数据库连接实例
/// * `sql` - 要执行的删除 SQL 语句
/// 
/// # 返回值
/// * `Ok(u64)` - 成功时返回受影响的行数
/// * `Err(String)` - 失败时返回错误信息
/// 
/// # 错误处理
/// 会记录执行错误的日志，并将错误信息转换为字符串返回
async fn execute_delete(db: &DBConn, sql: &str) -> Result<u64, String> {
    match db.delete(sql).await {
        Ok(row) => Ok(row),
        Err(err) => {
            log::error!("sql.delete error {} {:?}", sql, err);
            Err(err.to_string())
        }
    }
}