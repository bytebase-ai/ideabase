pub mod core;
pub mod cache;

// 表元数据
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TableMeta {
    // 数据库名
    pub schema: String,
    // 表名
    pub name: String,
    // 字段名 -> 字段元数据
    pub columns: fnv::FnvHashMap<String, ColumnMeta>,
    // 表注释
    pub comment: Option<String>,
}

// 字段元数据
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ColumnMeta {
    // 字段名
    pub field: String,
    // 字段类型
    pub type_name: String,
    // 是否为空
    pub null: Option<String>,
    // 默认值
    pub default: Option<String>,
    // 字段注释
    pub comment: Option<String>,
    // 索引类型
    pub key: Option<String>,
    // 额外信息
    pub extra: Option<String>,
}

use sqlx::Row;
/// 实现 sqlx::FromRow trait，用于将 MySQL 查询结果行转换为 ColumnMeta 结构体
/// 
/// 该实现负责从数据库查询结果中提取列的元数据信息，包括字段名、类型、约束等信息
impl<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> for ColumnMeta {
    /// 从数据库行数据转换为 ColumnMeta 实例
    /// 
    /// # 参数
    /// - `row`: 数据库查询结果行引用
    /// 
    /// # 返回值
    /// - `Result<Self, sqlx::Error>`: 成功返回 ColumnMeta 实例，失败返回错误信息
    /// 
    /// # 处理逻辑
    /// - 逐个提取行中的字段并转换为 ColumnMeta 的对应属性
    /// - 对于 BLOB 类型的字段（如 Type 和 Comment），需要先获取字节数据再转换为字符串
    fn from_row(row: &'r sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            // 获取字段名
            field: row.try_get("Field")?,
            // 获取字段类型（BLOB类型，需要转换为字符串）
            type_name: { // BLOB
                let bytes: Vec<u8> = row.try_get("Type")?;
                String::from_utf8(bytes).map_err(|e| sqlx::Error::Decode(e.into()))?
            },
            // 获取是否允许为空的设置（可选字段）
            null: row.try_get("Null").ok(),
            // 获取索引类型信息（BLOB类型，需要转换为字符串）
            key: {
                let bytes: Vec<u8> = row.try_get("Key")?;
                Some(String::from_utf8(bytes).map_err(|e| sqlx::Error::Decode(e.into()))?)
            },
            // 获取默认值（可选字段）
            default: row.try_get("Default").ok(),
            // 获取额外信息（可选字段）
            extra: row.try_get("Extra").ok(),
            // 获取字段注释（BLOB类型，需要转换为字符串）
            comment: { // BLOB
                let bytes: Vec<u8> = row.try_get("Comment")?;
                Some(String::from_utf8(bytes).map_err(|e| sqlx::Error::Decode(e.into()))?)
            },
        })
    }
}