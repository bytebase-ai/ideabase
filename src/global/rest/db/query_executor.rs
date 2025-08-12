use std::collections::HashMap;
use crate::global::db::core::{get_table, DBConn};

pub const DEFAULT_MAX_COUNT: usize = 10;

#[derive(Debug, Clone)]
pub struct QueryExecutor {
    schema: String,
    table: String,
    columns: Vec<String>,
    where_clauses: Vec<String>,
    params: Vec<serde_json::Value>,
    order: Option<String>,
    page: i32,
    limit: i32,
}

impl QueryExecutor {
    pub fn new() -> Self {
        QueryExecutor {
            schema: String::new(),
            table: String::new(),
            columns: vec![],
            where_clauses: vec![],
            params: vec![],
            order: None,
            page: 0,
            limit: 1,
        }
    }

    pub async fn exec(&self, db: &DBConn) -> Result<Vec<HashMap<String, serde_json::Value>>, sqlx::Error> {
        let sql = self.to_sql();
        log::info!("sql.exec: {}, params: {}", sql, serde_json::to_string(&self.params).unwrap());
        let params: Vec<String> = self.params.iter()
            .map(|v| match v {
                serde_json::Value::Null => "NULL".to_string(),
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(_) | serde_json::Value::Object(_) =>
                    serde_json::to_string(v).unwrap_or_else(|_| "NULL".to_string()),
                _ => v.to_string(),
            })
            .collect();
        db.query_list(&sql, params).await
    }

    pub fn to_sql(&self) -> String {
        let mut sql = String::from("SELECT ");
        if self.columns.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&self.columns.join(","));
        }
        sql.push_str(" FROM ");
        sql.push_str(format!("{}.{}", self.schema, self.table).as_str());
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        if let Some(order) = &self.order {
            sql.push_str(" ORDER BY ");
            sql.push_str(order);
        }

        if self.limit > 0 {
            sql.push_str(" LIMIT ");
            sql.push_str(&self.limit.to_string());

            sql.push_str(" OFFSET ");
            sql.push_str(&(self.limit * self.page).to_string());
        }
        sql
    }
    
    pub fn parse_table(&mut self, table_key: &str) -> Result<(), String> {
        let table_key = if table_key.ends_with("[]") { &table_key[..table_key.len()-2] } else { table_key };
        let schema_table_vec = table_key.split(".").collect::<Vec<&str>>();
        let schema = schema_table_vec[0];
        let table = schema_table_vec[1];
        match get_table(&schema, table) {
            Some(table) => {
                self.table = table.name.clone();
                self.schema = table.schema.clone();
                Ok(())
            },
            None => Err(format!("table: {} not exists", table_key))
        }
    }

    pub fn parse_condition(&mut self, field: &str, value: &serde_json::Value) {
        if field.starts_with('@') {
            match &field[1..] {
                "order" => {
                    if let serde_json::Value::String(order) = value {
                        self.order = Some(order.to_string());
                    }
                }
                "column" => {
                    if let serde_json::Value::String(cols) = value {
                        self.columns.extend(cols.split(',').map(|s| s.trim().to_string().to_lowercase()));
                    }
                }
                _ => {}
            }
            return;
        }
        if field.ends_with('$') {
            let actual_field = &field[..field.len() - 1];
            self.where_clauses.push(format!("{} LIKE ?", actual_field));
            self.params.push(value.to_owned());
            return;
        }
        match value {
            serde_json::Value::Array(values) => {
                let mut condition = format!("{} in (", field);
                let placeholders: Vec<_> = (0..values.len())
                    .map(|i| if i == 0 { "?" } else { ",?" })
                    .collect();
                condition.push_str(&placeholders.join(""));
                condition.push(')');
                self.where_clauses.push(condition);
                self.params.extend(values.to_owned());
            }
            _ => {
                self.where_clauses.push(format!("{}=?", field));
                self.params.push(value.to_owned());
            }
        }
    }

    pub fn page_size(&mut self, page: serde_json::Value, count: serde_json::Value) {
        self.page = Self::parse_num(&page, 0);
        self.limit = Self::parse_num(&count, 10);
    }

    fn parse_num(value: &serde_json::Value, default_val: i32) -> i32 {
        match value {
            serde_json::Value::Number(n) => n.as_f64()
                .map(|f| f as i32)
                .unwrap_or(default_val),
            _ => default_val,
        }
    }

    pub fn add_column(&mut self, column: &str) {
        // *代替，必然包含所有字段
        if self.columns.is_empty() { return; }
        // 包含当前字段，跳过
        if self.columns.iter().any(|c| c.eq(column)) { return; }
        self.columns.push(column.to_string());
    }
}