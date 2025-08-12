use lazy_static::lazy_static;
use std::sync::RwLock;
use std::collections::HashMap;
use fnv::FnvHashMap;
use sqlx::{mysql::{MySqlColumn, MySqlRow, MySqlPool}, Column, Row, TypeInfo, types::Decimal};
use crate::global::common::utils::base64_encode;
use crate::global::db::{ColumnMeta, DbMeta, TableMeta};

lazy_static! {
    static ref DB_MAP: RwLock<FnvHashMap<String, DbMeta>> = RwLock::new(FnvHashMap::default());
    static ref DB_TABLES_MAP: RwLock<FnvHashMap<String, Vec<String>>> = RwLock::new(FnvHashMap::default());
    static ref TABLE_MAP: RwLock<FnvHashMap<String, TableMeta>> = RwLock::new(FnvHashMap::default());
}

// MySQL系统数据库列表`
const MYSQL_SYS_DB: &[&str] = &["information_schema", "mysql", "performance_schema", "sys"];

#[derive(Debug, Clone)]
pub struct DBConn {
    pool: MySqlPool,
}

impl DBConn {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(url).await?;
        let mut ds = Self { pool };
        ds.init().await?;
        Ok(ds)
    }

    async fn init(&mut self) -> Result<(), sqlx::Error> {
        self.load_db().await?;

        let db_names = {
            let db_map = DB_MAP.read().unwrap();
            db_map.keys().cloned().collect::<Vec<_>>()
        };

        for db_name in db_names {
            self.load_db_table(&db_name).await?;
        }

        Ok(())
    }

    async fn load_db(&mut self) -> Result<(), sqlx::Error> {
        let list_db_sql = "SELECT table_schema AS name,
                          ROUND(SUM(data_length + index_length) / 1024 / 1024, 2) AS size
                          FROM information_schema.tables
                          GROUP BY table_schema;";

        let db_list = sqlx::query(list_db_sql).fetch_all(&self.pool).await?;
        let mut all_dbs = DB_MAP.write().unwrap();

        for db_row in db_list.iter() {
            let db_name: String = match db_row.try_get("name") {
                Ok(name) => name,
                Err(_) => {
                    let bytes: Vec<u8> = db_row.get("name");
                    String::from_utf8(bytes).unwrap_or_default()
                }
            };

            if MYSQL_SYS_DB.contains(&db_name.as_str()) {
                continue;
            }

            let db_size: Decimal = db_row.get("size");
            let size = db_size.to_string().parse::<f64>().unwrap_or(0.0);

            all_dbs.insert(
                db_name.clone(),
                DbMeta { name: db_name, size }
            );
        }

        Ok(())
    }

    async fn load_db_table(&mut self, schema: &str) -> Result<(), sqlx::Error> {
        let list_db_table_sql = format!(
            "SELECT TABLE_NAME, TABLE_COMMENT
             FROM information_schema.tables
             WHERE table_schema='{}' AND table_type='BASE TABLE'",
            schema
        );

        let tables = sqlx::query(&list_db_table_sql).fetch_all(&self.pool).await?;
        let mut table_name_list = Vec::with_capacity(tables.len());

        {
            let mut all_tables = TABLE_MAP.write().unwrap();

            for table_row in tables {
                let table_name: String = match table_row.try_get("TABLE_NAME") {
                    Ok(name) => name,
                    Err(_) => {
                        let bytes: Vec<u8> = table_row.get("TABLE_NAME");
                        String::from_utf8(bytes).unwrap_or_default()
                    }
                };
                let table_comment: String = match table_row.try_get("TABLE_COMMENT") {
                    Ok(name) => name,
                    Err(_) => {
                        let bytes: Vec<u8> = table_row.get("TABLE_COMMENT");
                        String::from_utf8(bytes).unwrap_or_default()
                    }
                };

                let columns = self.load_table_meta(schema, &table_name).await?;
                let table_meta = TableMeta {
                    schema: schema.to_string(),
                    name: table_name.clone(),
                    columns,
                    comment: Some(table_comment)
                };

                let table_key = format!("{}.{}", schema, &table_name);
                log::info!("mysql.table: {} loaded", &table_key);

                all_tables.insert(table_key, table_meta);
                table_name_list.push(table_name);
            }
        }

        let mut db_tables = DB_TABLES_MAP.write().unwrap();
        db_tables.insert(schema.to_string(), table_name_list);

        Ok(())
    }

    async fn load_table_meta(&self, schema: &str, table_name: &str) -> Result<FnvHashMap<String, ColumnMeta>, sqlx::Error> {
        let columns: Vec<ColumnMeta> = sqlx::query_as(&format!("SHOW FULL COLUMNS FROM {}.{}", schema, table_name))
            .fetch_all(&self.pool).await?;
        let mut column_map = FnvHashMap::default();
        for column in columns {
            column_map.insert(column.field.clone(), column);
        }
        Ok(column_map)
    }

    pub async fn query_one(&self, sql: &str, params: Vec<String>) -> Result<Option<HashMap<String, serde_json::Value>>, sqlx::Error> {
        let sql = if !sql.to_lowercase().contains("limit") {
            format!("{} limit 1", sql)
        } else {
            sql.to_string()
        };

        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }
        let row_opt = query.fetch_optional(&self.pool).await?;

        if let Some(row) = row_opt {
            let columns = row.columns();
            let mut record = HashMap::with_capacity(columns.len());
            for column in columns {
                let value = Self::get_column_val(&row, column);
                record.insert(column.name().to_string(), value);
            }
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    pub async fn query_list(&self, sql: &str, params: Vec<String>) -> Result<Vec<HashMap<String, serde_json::Value>>, sqlx::Error> {
        let mut query = sqlx::query(sql);
        for param in params {
            query = query.bind(param);
        }
        let rows = query.fetch_all(&self.pool).await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows.into_iter() {
            let mut record = HashMap::with_capacity(row.columns().len());
            for column in row.columns() {
                let value = Self::get_column_val(&row, column);
                record.insert(column.name().to_string(), value);
            }
            results.push(record);
        }

        Ok(results)
    }

    fn get_column_val(row: &MySqlRow, column: &MySqlColumn) -> serde_json::Value {
        match column.type_info().name() {
            "BIGINT" | "INT" => row.try_get::<i64, _>(column.name()).map_or(serde_json::Value::Null, serde_json::Value::from),
            "DATETIME" | "TIMESTAMP" | "DATE" | "TIME" => row.try_get::<chrono::NaiveDateTime, _>(column.name())
                .map_or(serde_json::Value::Null, |val| serde_json::Value::String(val.to_string())),
            "TINYTEXT" | "TEXT" | "MEDIUMTEXT" | "LONGTEXT" => row.try_get::<String, _>(column.name())
                .map_or(serde_json::Value::Null, serde_json::Value::String),
            "JSON" => row.try_get::<serde_json::Value, _>(column.name())
                .unwrap_or(serde_json::Value::Null),
            "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "VARBINARY" | "BINARY" => {
                row.try_get::<Vec<u8>, _>(column.name()).map_or_else(
                    |_| serde_json::Value::Null,
                    |bytes| match String::from_utf8(bytes.clone()) {
                        Ok(s) => serde_json::Value::String(s),
                        Err(_) => serde_json::Value::String(base64_encode(bytes)),
                    }
                )
            }
            "DECIMAL" => {
                row.try_get::<Decimal, _>(column.name())
                    .map(|decimal| serde_json::Value::String(decimal.to_string()))
                    .map_err(|err| {
                        log::error!("DECIMAL.getError: failed to decode column \"{}\": {}", column.name(), err);
                    })
                    .unwrap_or(serde_json::Value::Null)
            }
            "FLOAT" | "DOUBLE" => {
                row.try_get::<f64, _>(column.name())
                    .map_or_else(
                        |err| {
                            log::error!("FLOAT/DOUBLE.getError: failed to decode column \"{}\": {}", column.name(), err);
                            serde_json::Value::Null
                        },
                        serde_json::Value::from
                    )
            }
            _ => row.try_get::<String, _>(column.name())
                .map_or(serde_json::Value::Null, serde_json::Value::from),
        }
    }

    pub async fn insert(&self, sql: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(sql)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn update(&self, sql: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(sql)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete(&self, sql: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(sql)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn count(&self, sql: &str, params: Vec<String>) -> Result<i64, sqlx::Error> {
        let mut query_scalar = sqlx::query_scalar::<_, i64>(sql);
        for param in params {
            query_scalar = query_scalar.bind(param);
        }
        let count = query_scalar.fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn create_table(&self, sql: &str) -> Result<(), sqlx::Error> {
        sqlx::query(sql)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

pub fn is_table_exists(schema: &str, table: &str) -> bool {
    let table_key = format!("{schema}.{table}");
    TABLE_MAP.read().unwrap().contains_key(&table_key)
}

pub fn get_table(schema: &str, table: &str) -> Option<TableMeta> {
    let table_key = format!("{schema}.{table}");
    TABLE_MAP.read().unwrap().get(&table_key).cloned()
}

pub fn get_table_name_list(schema: &str) -> HashMap<String, String> {
    let db_tables = DB_TABLES_MAP.read().unwrap();
    let tables = match db_tables.get(schema) {
        Some(t) => t,
        None => return HashMap::new(),
    };

    let all_tables = TABLE_MAP.read().unwrap();
    tables.iter()
        .filter_map(|table_name| {
            all_tables.get(table_name.as_str())
                .map(|table| {
                    let comment = table.comment.as_deref().unwrap_or("");
                    (table_name.clone(), comment.to_string())
                }
            )
        }
    ).collect()
}
