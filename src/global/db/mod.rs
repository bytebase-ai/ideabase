use sqlx::Row;
use crate::global::db::core::DBConn;

pub mod core;

// 初始化数据库连接池
pub async fn init_datasource_conn(url: &str) -> Result<DBConn, sqlx::Error> {
    DBConn::new(url).await
}

// 数据库元数据
pub struct DbMeta {
    // 数据库名
    pub name: String,
    // 磁盘大小
    pub size: f64,
}

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
impl<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> for ColumnMeta {
    fn from_row(row: &'r sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            field: row.try_get("Field")?,
            type_name: { // BLOB
                let bytes: Vec<u8> = row.try_get("Type")?;
                String::from_utf8(bytes).map_err(|e| sqlx::Error::Decode(e.into()))?
            },
            null: row.try_get("Null").ok(),
            key: {
                let bytes: Vec<u8> = row.try_get("Key")?;
                Some(String::from_utf8(bytes).map_err(|e| sqlx::Error::Decode(e.into()))?)
            },
            default: row.try_get("Default").ok(),
            extra: row.try_get("Extra").ok(),
            comment: { // BLOB
                let bytes: Vec<u8> = row.try_get("Comment")?;
                Some(String::from_utf8(bytes).map_err(|e| sqlx::Error::Decode(e.into()))?)
            },
        })
    }
}