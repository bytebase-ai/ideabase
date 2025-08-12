use std::{collections::HashMap, vec};
use crate::global::db::core::DBConn;
use serde::Deserialize;
use sqlx::{Error, Result};

use crate::global::common::utils::get_next_id;

pub const IB_SCHEMA_NAME: &str = "ideabase";

/// 用户账户信息结构体
#[derive(Debug, sqlx::FromRow)]
pub struct Account {
    /// 用户唯一ID
    pub id: i64,
    /// 用户邮箱地址
    pub email: String,
    /// 用户密码(加密后)
    pub password: String,
    /// 用户角色(admin/user等)
    pub role: String,
    /// 用户API密钥
    pub api_key: Option<String>,
    /// 邮箱确认时间
    pub email_confirmed_at: Option<String>,
    /// 最后登录时间
    pub last_sign_in_at: Option<String>,
    /// 记录创建时间
    pub gmt_create: String,
    /// 记录更新时间
    pub gmt_update: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountDTO {
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
    pub api_key: Option<String>,
    pub email_confirmed_at: Option<String>,
    pub last_sign_in_at: Option<String>,
    pub gmt_create: Option<String>,
    pub gmt_update: Option<String>,
}

impl Account {
    pub async fn create(db_conn: &DBConn, dto: &AccountDTO) -> Result<i64, Error> {
        let account_id = get_next_id();

        // 构建字段名部分
        let mut columns = vec!["id".to_string()];
        let mut values = vec![account_id.to_string()];
        
        // 定义要检查的字段及其对应的值
        let fields = [
            ("email", &dto.email),
            ("password", &dto.password),
            ("role", &dto.role),
            ("api_key", &dto.api_key),
            ("email_confirmed_at", &dto.email_confirmed_at),
            ("last_sign_in_at", &dto.last_sign_in_at),
            ("gmt_create", &dto.gmt_create),
            ("gmt_update", &dto.gmt_update),
        ];
        
        // 收集存在的字段名和值
        for (field_name, field_value) in &fields {
            if let Some(value) = field_value {
                columns.push(ToString::to_string(field_name));
                values.push(format!("'{}'", value.replace("'", "''")));
            }
        }
        
        // 构建完整SQL语句
        let create_sql = format!("INSERT INTO `{IB_SCHEMA_NAME}`.`account` ({}) VALUES ({})", columns.join(", "), values.join(", "));
        log::info!("account.create.SQL: {}", create_sql);
        db_conn.insert(&create_sql).await.map(|_| account_id)
    }

    pub async fn update(db_conn: &DBConn, account_id: i64, dto: &AccountDTO) -> Result<i64, Error> {
        // 定义要更新的字段及其对应的值
        let fields = [
            ("email", &dto.email),
            ("password", &dto.password),
            ("role", &dto.role),
            ("api_key", &dto.api_key),
            ("email_confirmed_at", &dto.email_confirmed_at),
            ("last_sign_in_at", &dto.last_sign_in_at),
            ("gmt_update", &dto.gmt_update),
        ];
        
        // 构建SET子句
        let mut set_clauses = Vec::new();
        for (field_name, field_value) in &fields {
            if let Some(value) = field_value {
                set_clauses.push(format!("{} = '{}'", field_name, value.replace("'", "''")));
            }
        }
        
        // 如果没有更新，直接返回id 
        if set_clauses.is_empty() { 
            return Err(Error::Configuration("No fields to update".into()));
        }
        
        // 构建完整SQL语句
        let update_sql = format!("UPDATE `{IB_SCHEMA_NAME}`.`account` SET {} WHERE id = {}", set_clauses.join(", "), account_id);
        log::info!("account.update.SQL: {}", update_sql);
        
        // 执行更新并处理结果
        db_conn.update(&update_sql).await.map(|cnt| if cnt > 0 { account_id } else { -1 })
    }

    pub async fn fetch_by_id(db_conn: &DBConn, account_id: i64) -> Result<Account, Error> {
        let select_sql = format!("SELECT * FROM `{IB_SCHEMA_NAME}`.`account` WHERE id = ?");
        log::info!("account.fetch.SQL: {}", select_sql);
        let fetch_result = db_conn.query_one(&select_sql, vec![account_id.to_string()]).await?;
        let record = fetch_result.ok_or(Error::RowNotFound)?;
        Ok(Self::from_record(&record))
    }

    pub async fn fetch_by_email(db_conn: &DBConn, email: &str) -> Result<Account, Error> {
        let select_sql = format!("SELECT * FROM `{IB_SCHEMA_NAME}`.`account` WHERE email = ?");
        let fetch_result = db_conn.query_one(&select_sql, vec![email.to_string()]).await?;
        let record = fetch_result.ok_or(Error::RowNotFound)?;
        Ok(Self::from_record(&record))
    }

    pub async fn count_by_email(db_conn: &DBConn, email: &str) -> Result<i64, Error> {
        let count_sql = format!("SELECT count(1) FROM `{IB_SCHEMA_NAME}`.`account` WHERE email = ?");
        log::info!("account.count_by_email.SQL: {}", count_sql);
        let cnt = db_conn.count(&count_sql, vec![email.to_string()]).await?;
        Ok(cnt)
    }

    fn from_record(record: &HashMap<String, serde_json::Value>) -> Self {
        Account {
            id: record.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            email: record.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            password: record.get("password").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            role: record.get("role").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            api_key: record.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string()),
            email_confirmed_at: record.get("email_confirmed_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
            last_sign_in_at: record.get("last_sign_in_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
            gmt_create: record.get("gmt_create").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            gmt_update: record.get("gmt_update").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }
    }
}



use std::fmt::{self, Display};
use std::str::FromStr;

/// 用户角色枚举
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Role {
    Admin,
    User,
}
impl Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::User => write!(f, "user"),
        }
    }
}
impl FromStr for Role {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            _ => Err(Error::Decode("Invalid role string".into())),
        }
    }
}



#[cfg(test)]
mod tests {
    use crate::global::common::log::init_tk_log;
    use super::*;

    #[tokio::test]
    async fn test_create_account() {
        init_tk_log();

        let dto = AccountDTO {
            email: Some("test@example.com".to_string()),
            password: Some("hashed_password".to_string()),
            role: Some("user".to_string()),
            api_key: None,
            email_confirmed_at: None,
            last_sign_in_at: None,
            gmt_create: None,
            gmt_update: None,
        };

        if let Ok(mysql_url) = std::env::var("MYSQL_URL") {
            let db_conn = DBConn::new(&mysql_url).await.unwrap();
            let account_id = Account::create(&db_conn, &dto).await;
            match account_id {
                Ok(account_id) => {
                    println!("account.create: {}", account_id);
                    // fetch.0
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    test_fetch_by_id(account_id, &db_conn).await;
                    // update
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    test_update_account(account_id, &db_conn).await;
                    // fetch.1
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    test_fetch_by_id(account_id, &db_conn).await;
                }
                Err(err) => {
                    eprintln!("account.create error {}", err);
                }
            }
        } else {
            eprintln!("Error getting MySQL_URL");
        }
    }

    async fn test_update_account(account_id: i64, db_conn: &DBConn) {
        // 准备测试数据
        let dto = AccountDTO {
            email: Some("updated@example.com".to_string()),
            password: Some("new_hashed_password".to_string()),
            role: None,
            api_key: None,
            email_confirmed_at: None,
            last_sign_in_at: None,
            gmt_create: None,
            gmt_update: Some("2023-01-01".to_string()),
        };
        let update_result = Account::update(db_conn, account_id, &dto).await;
        println!("account.update: {} {:#?}", account_id, update_result);
    }

    async fn test_fetch_by_id(account_id: i64, db_conn: &DBConn) {
        let result = Account::fetch_by_id(db_conn, account_id).await;
        println!("account.feetch: {} {:#?}", account_id, result);
    }
}
