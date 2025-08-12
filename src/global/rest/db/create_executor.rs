use crate::global::db::core::DBConn;
use crate::global::db::TableMeta;

#[derive(Debug, Clone)]
pub struct CreateExecutor {
    pub table_meta: TableMeta,
}

impl CreateExecutor {
    pub fn new(table_meta: TableMeta) -> Self {
        CreateExecutor { table_meta }
    }

    pub async fn create_table(&self, db: &DBConn) -> Result<(), sqlx::Error> {
        let create_sql = get_create_table_sql(&self.table_meta);
        db.create_table(&create_sql).await
    }
}


// 根据TableMeta生成create table语句
pub fn get_create_table_sql(table: &TableMeta) -> String {
    let mut sql = format!("CREATE TABLE `{}` (\n", table.name);
    // 添加列定义
    for (i, (col_name, col_meta)) in table.columns.iter().enumerate() {
        sql.push_str(&format!("  `{}` {}", col_name, col_meta.type_name));

        // 添加列属性
        if let Some(_null) = &col_meta.null {
            if _null == "NO" {
                sql.push_str(" NOT NULL");
            }
        }
        if let Some(default) = &col_meta.default {
            sql.push_str(&format!(" DEFAULT '{}'", default));
        }
        if let Some(comment) = &col_meta.comment {
            sql.push_str(&format!(" COMMENT '{}'", comment));
        }
        if let Some(extra) = &col_meta.extra {
            if extra.contains("auto_increment") {
                sql.push_str(" AUTO_INCREMENT");
            }
        }
        // 如果不是最后一列，添加逗号
        if i < table.columns.len() - 1 {
            sql.push_str(",\n");
        }
    }

    // 添加主键定义
    let primary_keys: Vec<_> = table.columns.iter()
        .filter(|(_, meta)| meta.key.as_ref().map_or(false, |k| k == "PRI"))
        .map(|(name, _)| format!("`{}`", name))
        .collect();

    if !primary_keys.is_empty() {
        sql.push_str(",\n");
        sql.push_str(&format!("  PRIMARY KEY ({})", primary_keys.join(", ")));
    }

    // 添加唯一键定义
    let unique_keys: Vec<_> = table.columns.iter()
        .filter(|(_, meta)| meta.key.as_ref().map_or(false, |k| k == "UNI"))
        .map(|(name, _)| format!("`{}`", name))
        .collect();

    if !unique_keys.is_empty() {
        sql.push_str(",\n");
        sql.push_str(&format!("  UNIQUE KEY ({})", unique_keys.join(", ")));
    }

    // 添加索引键定义
    let index_keys: Vec<_> = table.columns.iter()
        .filter(|(_, meta)| meta.key.as_ref().map_or(false, |k| k == "MUL"))
        .map(|(name, _)| format!("`{}`", name))
        .collect();

    if !index_keys.is_empty() {
        sql.push_str(",\n");
        sql.push_str(&format!("  INDEX ({})", index_keys.join(", ")));
    }

    sql.push_str("\n)");

    // 添加表注释
    if let Some(comment) = &table.comment {
        sql.push_str(&format!(" COMMENT='{}'", comment));
    }
    sql.push(';');
    sql
}

#[cfg(test)]
mod tests {
    use crate::global::db::TableMeta;
    use crate::global::rest::db::create_executor::get_create_table_sql;

    const TABLE_META_JSON: &str = r#" {
  "name": "account",
  "columns": {
    "phone_num": {
      "field": "phone_num",
      "type_name": "varchar(16)",
      "null": "NO",
      "default": null,
      "comment": "手机号",
      "key": "UNI",
      "extra": ""
    },
    "id": {
      "field": "id",
      "type_name": "bigint",
      "null": "NO",
      "default": null,
      "comment": "账号ID",
      "key": "PRI",
      "extra": ""
    },
    "from_act_id": {
      "field": "from_act_id",
      "type_name": "bigint",
      "null": "YES",
      "default": null,
      "comment": "邀请者ID",
      "key": "MUL",
      "extra": ""
    },
    "gmt_create": {
      "field": "gmt_create",
      "type_name": "datetime",
      "null": "NO",
      "default": null,
      "comment": "创建时间",
      "key": "",
      "extra": "DEFAULT_GENERATED"
    }
  },
  "comment": null
}
"#;

    #[test]
    fn test_create_table_sql() {
        let table_meta: TableMeta = serde_json::from_str(TABLE_META_JSON).unwrap();
        let sql = get_create_table_sql(&table_meta);
        println!("{}", sql);
    }
}

