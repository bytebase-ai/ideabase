use fnv::FnvHashMap;
use sqlx::{
    Column, Row, TypeInfo,
    mysql::{MySqlColumn, MySqlPool, MySqlRow},
    types::Decimal,
};
use std::collections::HashMap;

use crate::global::common::utils::base64_encode;
use crate::global::db::{ColumnMeta, TableMeta};

// MySQL系统数据库列表`
const MYSQL_SYS_DB: &[&str] = &["information_schema", "mysql", "performance_schema", "sys"];

/// 数据库连接结构体
///
/// 用于管理 MySQL 数据库连接池，提供数据库操作的基础功能。
///
/// # 字段
/// * `pool` - MySQL 连接池，用于执行数据库操作
#[derive(Debug, Clone)]
pub struct DBConn {
    /// MySQL 连接池
    pool: MySqlPool,
}

impl DBConn {
    /// 创建一个新的数据库连接实例
    ///
    /// 该方法会建立 MySQL 连接池，并初始化数据库元数据信息。
    ///
    /// # 参数
    /// * `url` - MySQL 数据库连接 URL
    ///
    /// # 返回值
    /// * `Ok(Self)` - 成功创建的数据库连接实例
    /// * `Err(sqlx::Error)` - 连接或初始化过程中发生的错误
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(url).await?;
        let mut ds = Self { pool };
        ds.init().await?;
        Ok(ds)
    }

    /// 初始化数据库连接
    ///
    /// 加载所有用户数据库及其表结构信息到内存缓存中。
    ///
    /// # 返回值
    /// * `Ok(())` - 初始化成功
    /// * `Err(sqlx::Error)` - 初始化过程中发生的错误
    async fn init(&mut self) -> Result<(), sqlx::Error> {
        let db_names = self.load_db().await?;
        for db_name in db_names {
            self.load_db_table(&db_name).await?;
        }
        Ok(())
    }

    /// 加载数据库列表
    ///
    /// 从 MySQL 的 information_schema 中查询所有用户数据库的名称和大小，
    /// 过滤掉系统数据库，并将数据库信息写入全局缓存。
    ///
    /// # 返回值
    /// * `Ok(Vec<String>)` - 成功加载的数据库名称列表
    /// * `Err(sqlx::Error)` - 查询或处理过程中发生的错误
    async fn load_db(&mut self) -> Result<Vec<String>, sqlx::Error> {
        // 查询所有数据库的名称和大小（单位：MB）
        let list_db_sql = "SELECT table_schema AS name, ROUND(SUM(data_length + index_length) / 1024 / 1024, 2) AS size
                        FROM information_schema.tables GROUP BY table_schema;";

        let db_list = sqlx::query(list_db_sql).fetch_all(&self.pool).await?;
        let mut db_names = Vec::with_capacity(db_list.len());

        for db_row in db_list.iter() {
            // 尝试获取数据库名称，如果失败则尝试从字节序列转换
            let db_name: String = match db_row.try_get("name") {
                Ok(name) => name,
                Err(_) => {
                    let bytes: Vec<u8> = db_row.get("name");
                    String::from_utf8(bytes).unwrap_or_default()
                }
            };

            // 跳过 MySQL 系统数据库
            if MYSQL_SYS_DB.contains(&db_name.as_str()) {
                continue;
            }

            // 获取数据库大小并转换为 f64 类型
            let db_size: Decimal = db_row.get("size");
            let db_size = db_size.to_string().parse::<f64>().unwrap_or(0.0);

            // 使用外部缓存替代内部静态变量
            crate::global::db::cache::put_db_meta(db_name.clone(), db_size);

            // 将数据库名称添加到结果列表中
            db_names.push(db_name);
        }

        Ok(db_names)
    }
    
    /// 加载指定数据库中的所有表信息
    ///
    /// 该方法会查询指定数据库中的所有基础表（BASE TABLE），获取表名和表注释，
    /// 然后加载每个表的列元数据信息，并将这些信息存储到外部缓存中。
    ///
    /// # 参数
    /// * `schema` - 数据库名称
    ///
    /// # 返回值
    /// * `Ok(())` - 成功加载所有表信息
    /// * `Err(sqlx::Error)` - 查询或处理过程中发生的错误
    async fn load_db_table(&mut self, schema: &str) -> Result<(), sqlx::Error> {
        // 构造查询表信息的 SQL 语句，只查询基础表（BASE TABLE）
        let list_db_table_sql = format!(
            "SELECT TABLE_NAME, TABLE_COMMENT
            FROM information_schema.tables
            WHERE table_schema='{}' AND table_type='BASE TABLE'",
            schema
        );

        // 执行查询，获取所有表的信息
        let tables = sqlx::query(&list_db_table_sql)
            .fetch_all(&self.pool)
            .await?;
        
        // 预分配容量的表名列表，用于存储当前数据库中的所有表名
        let mut table_name_list = Vec::with_capacity(tables.len());

        // 遍历查询结果，构建每个表的元数据信息
        for table_row in tables {
            // 获取表名，如果直接获取失败，则尝试从字节序列转换
            let table_name: String = match table_row.try_get("TABLE_NAME") {
                Ok(name) => name,
                Err(_) => {
                    let bytes: Vec<u8> = table_row.get("TABLE_NAME");
                    String::from_utf8(bytes).unwrap_or_default()
                }
            };
            
            // 获取表注释，如果直接获取失败，则尝试从字节序列转换
            let table_comment: String = match table_row.try_get("TABLE_COMMENT") {
                Ok(name) => name,
                Err(_) => {
                    let bytes: Vec<u8> = table_row.get("TABLE_COMMENT");
                    String::from_utf8(bytes).unwrap_or_default()
                }
            };

            // 加载表的列元数据信息
            let columns = self.load_table_meta(schema, &table_name).await?;
            
            // 构建表元数据结构体
            let table_meta = TableMeta {
                schema: schema.to_string(),
                name: table_name.clone(),
                columns,
                comment: Some(table_comment),
            };

            // 记录表加载日志
            log::info!("mysql.table: {}.{} loaded", schema, &table_name);

            // 将表元数据存入外部缓存
            crate::global::db::cache::put_table_meta(schema, &table_name, table_meta);
            
            // 将表名添加到表名列表中
            table_name_list.push(table_name);
        }

        // 将当前数据库的表列表存入外部缓存
        crate::global::db::cache::put_db_tables(schema.to_string(), table_name_list);

        Ok(())
    }

    /// 加载表的列元数据信息
    ///
    /// 通过执行 `SHOW FULL COLUMNS` 语句获取指定表的所有列信息，
    /// 并将结果转换为 `ColumnMeta` 结构体列表，最终构建成以列名为键的哈希映射。
    ///
    /// # 参数
    /// * `schema` - 数据库名称
    /// * `table_name` - 表名称
    ///
    /// # 返回值
    /// * `Ok(FnvHashMap<String, ColumnMeta>)` - 成功加载的列元数据映射
    /// * `Err(sqlx::Error)` - 查询或处理过程中发生的错误
    async fn load_table_meta(&self, schema: &str, table_name: &str) -> Result<FnvHashMap<String, ColumnMeta>, sqlx::Error> {
        // 构造查询表列信息的 SQL 语句
        let sql = format!("SHOW FULL COLUMNS FROM `{}`.`{}`", schema, table_name);
        
        // 执行查询并将结果映射为 ColumnMeta 结构体列表
        let columns: Vec<ColumnMeta> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await?;
        
        // 创建一个预分配容量的哈希映射，用于存储列元数据
        let mut column_map = FnvHashMap::with_capacity_and_hasher(columns.len(), Default::default());
        
        // 遍历列信息，以列名作为键插入到哈希映射中
        for column in columns {
            column_map.insert(column.field.clone(), column);
        }
        
        Ok(column_map)
    }

    /// 查询单条记录
    ///
    /// 执行给定的 SQL 查询语句，返回第一条匹配的记录。
    /// 如果 SQL 语句中未包含 LIMIT 子句，会自动添加 `LIMIT 1` 以提高查询效率。
    ///
    /// # 参数
    /// * `sql` - 要执行的 SQL 查询语句
    /// * `params` - 查询参数列表，用于绑定到 SQL 语句中的占位符
    ///
    /// # 返回值
    /// * `Ok(Some(HashMap<String, serde_json::Value>))` - 成功查询到记录，返回包含字段名和值的映射
    /// * `Ok(None)` - 没有查询到记录
    /// * `Err(sqlx::Error)` - 查询过程中发生错误
    pub async fn query_one(&self, sql: &str, params: Vec<String>) -> Result<Option<HashMap<String, serde_json::Value>>, sqlx::Error> {
        // 如果 SQL 语句中没有包含 LIMIT，则自动添加 LIMIT 1 以提高查询效率
        let sql = if !sql.to_lowercase().contains("limit") {
            format!("{} LIMIT 1", sql)
        } else {
            sql.to_string()
        };

        // 构建查询并绑定参数
        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        // 执行查询并获取结果
        let row_opt = query.fetch_optional(&self.pool).await?;

        // 如果查询到记录，则将行数据转换为 HashMap
        match row_opt {
            Some(row) => {
                let columns = row.columns();
                let mut record = HashMap::with_capacity(columns.len());
                
                // 遍历所有列，将列名和对应的值插入到 HashMap 中
                for column in columns {
                    let value = Self::get_column_val(&row, column);
                    record.insert(column.name().to_string(), value);
                }
                
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// 查询多条记录
    ///
    /// 执行给定的 SQL 查询语句，返回所有匹配的记录列表。
    ///
    /// # 参数
    /// * `sql` - 要执行的 SQL 查询语句
    /// * `params` - 查询参数列表，用于绑定到 SQL 语句中的占位符
    ///
    /// # 返回值
    /// * `Ok(Vec<HashMap<String, serde_json::Value>>)` - 成功查询到的记录列表，每条记录为字段名和值的映射
    /// * `Err(sqlx::Error)` - 查询过程中发生错误
    pub async fn query_list(&self, sql: &str, params: Vec<String>) -> Result<Vec<HashMap<String, serde_json::Value>>, sqlx::Error> {
        // 构建查询并绑定参数
        let mut query = sqlx::query(sql);
        for param in params {
            query = query.bind(param);
        }

        // 执行查询并获取所有结果行
        let rows = query.fetch_all(&self.pool).await?;

        // 预分配容量的结果向量
        let mut results = Vec::with_capacity(rows.len());

        // 遍历每一行，将行数据转换为 HashMap 并添加到结果中
        for row in rows.into_iter() {
            let columns = row.columns();
            let mut record = HashMap::with_capacity(columns.len());
            
            // 遍历所有列，将列名和对应的值插入到 HashMap 中
            for column in columns {
                let value = Self::get_column_val(&row, column);
                record.insert(column.name().to_string(), value);
            }
            
            results.push(record);
        }

        Ok(results)
    }

    /// 获取列的值并转换为 JSON 值
    ///
    /// 根据 MySQL 列的类型信息，将数据库中的值转换为 `serde_json::Value`，
    /// 以便于后续的序列化和传输。
    ///
    /// # 参数
    /// * `row` - 当前行数据
    /// * `column` - 当前列信息
    ///
    /// # 返回值
    /// * `serde_json::Value` - 转换后的 JSON 值
    fn get_column_val(row: &MySqlRow, column: &MySqlColumn) -> serde_json::Value {
        let column_name = column.name();
        let type_name = column.type_info().name();

        match type_name {
            // 整数类型
            "BIGINT" | "INT" | "SMALLINT" | "MEDIUMINT" | "TINYINT" => {
                row.try_get::<i64, _>(column_name)
                    .map_or(serde_json::Value::Null, serde_json::Value::from)
            }
            // 日期时间类型
            "DATETIME" | "TIMESTAMP" => {
                row.try_get::<chrono::NaiveDateTime, _>(column_name)
                    .map_or(serde_json::Value::Null, |val| {
                        serde_json::Value::String(val.to_string())
                    })
            }
            "DATE" => {
                row.try_get::<chrono::NaiveDate, _>(column_name)
                    .map_or(serde_json::Value::Null, |val| {
                        serde_json::Value::String(val.to_string())
                    })
            }
            "TIME" => {
                row.try_get::<chrono::NaiveTime, _>(column_name)
                    .map_or(serde_json::Value::Null, |val| {
                        serde_json::Value::String(val.to_string())
                    })
            }
            // 文本类型
            "TINYTEXT" | "TEXT" | "MEDIUMTEXT" | "LONGTEXT" | "VARCHAR" | "CHAR" => {
                row.try_get::<String, _>(column_name)
                    .map_or(serde_json::Value::Null, serde_json::Value::String)
            }
            // JSON 类型
            "JSON" => {
                row.try_get::<serde_json::Value, _>(column_name)
                    .unwrap_or(serde_json::Value::Null)
            }
            // 二进制类型
            "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "VARBINARY" | "BINARY" => {
                row.try_get::<Vec<u8>, _>(column_name)
                    .map_or(serde_json::Value::Null, |bytes| {
                        // 尝试转换为 UTF-8 字符串，失败则进行 Base64 编码
                        match String::from_utf8(bytes.clone()) {
                            Ok(s) => serde_json::Value::String(s),
                            Err(_) => serde_json::Value::String(base64_encode(bytes)),
                        }
                    })
            }
            // 精确数值类型
            "DECIMAL" => {
                row.try_get::<Decimal, _>(column_name)
                    .map(|decimal| serde_json::Value::String(decimal.to_string()))
                    .unwrap_or_else(|err| {
                        log::error!(
                            "DECIMAL.getError: failed to decode column \"{}\": {}",
                            column_name,
                            err
                        );
                        serde_json::Value::Null
                    })
            }
            // 浮点数类型
            "FLOAT" | "DOUBLE" => {
                row.try_get::<f64, _>(column_name)
                    .map_or_else(
                        |err| {
                            log::error!(
                                "FLOAT/DOUBLE.getError: failed to decode column \"{}\": {}",
                                column_name,
                                err
                            );
                            serde_json::Value::Null
                        },
                        serde_json::Value::from,
                    )
            }
            // 布尔类型
            "BOOLEAN" | "BOOL" => {
                row.try_get::<bool, _>(column_name)
                    .map_or(serde_json::Value::Null, serde_json::Value::Bool)
            }
            // 其他未知类型默认转为字符串处理
            _ => {
                row.try_get::<String, _>(column_name)
                    .map_or(serde_json::Value::Null, serde_json::Value::from)
            }
        }
    }

    /// 插入数据
    ///
    /// 执行给定的 INSERT SQL 语句，返回受影响的行数。
    ///
    /// # 参数
    /// * `sql` - 要执行的 INSERT SQL 语句
    ///
    /// # 返回值
    /// * `Ok(i64)` - 成功插入数据，返回受影响的行数
    /// * `Err(sqlx::Error)` - 插入过程中发生错误
    pub async fn insert(&self, sql: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// 更新数据
    ///
    /// 执行给定的 UPDATE SQL 语句，返回受影响的行数。
    ///
    /// # 参数
    /// * `sql` - 要执行的 UPDATE SQL 语句
    ///
    /// # 返回值
    /// * `Ok(u64)` - 成功更新数据，返回受影响的行数
    /// * `Err(sqlx::Error)` - 更新过程中发生错误
    pub async fn update(&self, sql: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// 删除数据
    ///
    /// 执行给定的 DELETE SQL 语句，返回受影响的行数。
    ///
    /// # 参数
    /// * `sql` - 要执行的 DELETE SQL 语句
    ///
    /// # 返回值
    /// * `Ok(u64)` - 成功删除数据，返回受影响的行数
    /// * `Err(sqlx::Error)` - 删除过程中发生错误
    pub async fn delete(&self, sql: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// 查询记录数
    ///
    /// 执行给定的 COUNT SQL 语句，返回查询到的记录数。
    ///
    /// # 参数
    /// * `sql` - 要执行的 COUNT SQL 语句
    /// * `params` - 查询参数列表，用于绑定到 SQL 语句中的占位符
    ///
    /// # 返回值
    /// * `Ok(i64)` - 成功查询到记录数
    /// * `Err(sqlx::Error)` - 查询过程中发生错误
    pub async fn count(&self, sql: &str, params: Vec<String>) -> Result<i64, sqlx::Error> {
        let mut query_scalar = sqlx::query_scalar::<_, i64>(sql);
        for param in params {
            query_scalar = query_scalar.bind(param);
        }
        let count = query_scalar.fetch_one(&self.pool).await?;
        Ok(count)
    }

    /// 创建表
    ///
    /// 执行给定的 CREATE TABLE SQL 语句。
    ///
    /// # 参数
    /// * `sql` - 要执行的 CREATE TABLE SQL 语句
    ///
    /// # 返回值
    /// * `Ok(())` - 成功创建表
    /// * `Err(sqlx::Error)` - 创建表过程中发生错误
    pub async fn create_table(&self, sql: &str) -> Result<(), sqlx::Error> {
        sqlx::query(sql).execute(&self.pool).await?;
        Ok(())
    }
}
