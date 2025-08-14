use lazy_static::lazy_static;
use std::sync::RwLock;
use std::collections::HashMap;
use fnv::FnvHashMap;
use crate::global::db::TableMeta;

lazy_static! {
    static ref DB_CACHE: RwLock<FnvHashMap<String, f64>> = RwLock::new(FnvHashMap::default());
    static ref DB_TABLE_CACHE: RwLock<FnvHashMap<String, Vec<String>>> = RwLock::new(FnvHashMap::default());
    static ref TABLE_META_CACHE: RwLock<FnvHashMap<String, TableMeta>> = RwLock::new(FnvHashMap::default());
}

/// 向数据库缓存中添加或更新数据库元数据
/// 
/// # 参数
/// * `db_name` - 数据库名称
/// * `db_meta` - 数据库元数据
pub fn put_db_meta(db_name: String, db_size: f64) {
    if let Ok(mut cache) = DB_CACHE.write() {
        cache.insert(db_name, db_size);
    }
}

/// 向数据库表列表缓存中添加或更新数据库的表列表
/// 
/// # 参数
/// * `schema` - 数据库名称
/// * `table_list` - 表名列表
pub fn put_db_tables(schema: String, table_list: Vec<String>) {
    if let Ok(mut cache) = DB_TABLE_CACHE.write() {
        cache.insert(schema, table_list);
    }
}

/// 向表元数据缓存中添加或更新表的元数据
/// 
/// # 参数
/// * `schema` - 数据库名称
/// * `table` - 表名称
/// * `table_meta` - 表元数据
pub fn put_table_meta(schema: &str, table: &str, table_meta: TableMeta) {
    let table_key = format!("{}.{}", schema, table);
    if let Ok(mut cache) = TABLE_META_CACHE.write() {
        cache.insert(table_key, table_meta);
    }
}


/// 检查指定数据库中的表是否存在
/// 
/// # 参数
/// * `schema` - 数据库名称
/// * `table` - 表名称
/// 
/// # 返回值
/// 如果表存在返回 true，否则返回 false
pub fn is_table_exists(schema: &str, table: &str) -> bool {
    let table_key = format!("{}.{}", schema, table);
    TABLE_META_CACHE.read()
        .map(|guard| guard.contains_key(&table_key))
        .unwrap_or(false)
}

/// 获取指定数据库中表的元数据
/// 
/// # 参数
/// * `schema` - 数据库名称
/// * `table` - 表名称
/// 
/// # 返回值
/// 如果表存在，返回包含表元数据的 Some(TableMeta)，否则返回 None
pub fn get_table(schema: &str, table: &str) -> Option<TableMeta> {
    let table_key = format!("{}.{}", schema, table);
    TABLE_META_CACHE.read()
        .ok()?
        .get(&table_key)
        .cloned()
}

/// 获取数据库中所有表的名称和注释映射
/// 
/// # 参数
/// * `schema` - 数据库名称
/// 
/// # 返回值
/// 返回一个 HashMap，键为表名称，值为对应的注释字符串。
/// 如果数据库不存在或读取失败，返回空的 HashMap
pub fn get_table_name_list(schema: &str) -> HashMap<String, String> {
    let db_tables_guard = match DB_TABLE_CACHE.read() {
        Ok(guard) => guard,
        Err(_) => return HashMap::new(),
    };
    
    let tables = db_tables_guard.get(schema)?;
    
    let all_tables_guard = match TABLE_META_CACHE.read() {
        Ok(guard) => guard,
        Err(_) => return HashMap::new(),
    };
    
    tables.iter()
        .filter_map(|table_name| {
            all_tables_guard.get(table_name.as_str())
                .map(|table| {
                    let comment = table.comment.as_deref().unwrap_or("");
                    (table_name.clone(), comment.to_string())
                })
        })
        .collect()
}
