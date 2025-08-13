use regex::Regex;
use http::status::StatusCode;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use actix_web::{post, web, Responder};

use crate::global::common::rpc::RpcResult;
use crate::global::common::passwd::hash_passwd;
use crate::global::common::date::{format_datetime, now};
use crate::global::common::utils::generate_secure_api_key;
use crate::controller::build_rpc_response;
use crate::G_DB;
use crate::global::common::jwt::JwtToken;
use crate::service::model::account::{Account, AccountDTO, Role};

pub fn scope() -> actix_web::Scope {
    web::scope("/auth").service(logon).service(create).service(generate_api_key)
}

// 用户登录
#[post("/logon.json")]
async fn logon(request_data: web::Json<AccountRequest>) -> impl Responder {
    let account_request = request_data.into_inner();

    // 1. 验证输入参数
    let email = match account_request.email {
        Some(e) => e,
        None => return build_rpc_response(RpcResult::<serde_json::Value> { code: StatusCode::BAD_REQUEST, msg: Some("email must be provided".to_string()), payload: None }),
    };
    let password = match account_request.password {
        Some(p) => p,
        None => return build_rpc_response(RpcResult::<serde_json::Value> { code: StatusCode::BAD_REQUEST, msg: Some("password must be provided".to_string()), payload: None }),
    };

    // 2. 从数据库获取账户信息
    let db_conn = match G_DB.get() {
        Some(conn) => conn,
        None => {
            log::error!("Failed to get database connection");
            return build_rpc_response(RpcResult::<serde_json::Value> { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some("Database connection error".to_string()), payload: None });
        }
    };

    let account = match Account::fetch_by_email(db_conn, &email).await {
        Ok(acc) => acc, // Account found
        Err(err) => { // Account not found or database error
            log::error!("Failed to fetch account by email: {:?}", err);
            return build_rpc_response(RpcResult::<serde_json::Value> { code: StatusCode::UNAUTHORIZED, msg: Some("Invalid password".to_string()), payload: None });
        }
    };

    let account_id = account.id;
    let role = account.role;
    let hashed_password = account.password;
    // 3. 验证密码
    if !crate::global::common::passwd::verify_passwd(&password, &hashed_password) {
        return build_rpc_response(RpcResult::<serde_json::Value> { code: StatusCode::UNAUTHORIZED, msg: Some("Invalid password".to_string()), payload: None });
    }

    // 4. 生成 JWT Token
    let token = match JwtToken::new(account_id, &role).create_token() {
        Ok(t) => t,
        Err(err) => {
            log::error!("Failed to generate JWT token for account {}: {:?}", account_id, err);
            return build_rpc_response(RpcResult::<serde_json::Value> { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some("Failed to generate authentication token".to_string()), payload: None });
        }
    };

    // 5. 更新最后登录时间 (可选，但推荐)
    let update_dto = AccountDTO {
        last_sign_in_at: Some(format_datetime(now())),
        email: None, password: None, role: None, api_key: None, email_confirmed_at: None, gmt_create: None, gmt_update: None,
    };
    if let Err(err) = Account::update(db_conn, account_id, &update_dto).await {
        log::warn!("Failed to update last_sign_in_at for account {}: {:?}", account_id, err);
    }

    // 6. 返回成功响应和 Token
    let token_value = serde_json::json!({"token": token});
    build_rpc_response(RpcResult { code: StatusCode::OK, msg: None, payload: Some(token_value) })
}

// 创建普通用户 todo: admin账号才可以
#[post("/account.json")]
async fn create(request_data: web::Json<AccountRequest>) -> impl Responder {
    let mut account_request = request_data.into_inner();

    // 使用模式匹配进行参数验证
    match (account_request.email.as_ref(), account_request.password.as_ref()) {
        (None, _) => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("email must be provided".to_string()), payload: None }),
        (_, None) => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("password must be provided".to_string()), payload: None }),
        (Some(email), Some(password)) => {
            // 邮箱格式验证
            if !EMAIL_REGEX.is_match(email) {
                return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("invalid email format".to_string()), payload: None });
            }
            // 验证密码复杂度: 大小写、数字、8～32位
            let (mut has_upper, mut has_lower, mut has_digit) = (false, false, false);
            
            // 单次遍历检查所有字符条件
            password.chars().take(32).for_each(|c| {
                has_upper |= c.is_ascii_uppercase();
                has_lower |= c.is_ascii_lowercase();
                has_digit |= c.is_ascii_digit();
            });

            // 使用模式匹配处理所有验证条件: 大小写字母+长度(8~32)
            match (password.len(), has_upper, has_lower, has_digit) {
                (len, _, _, _) if len < 8 => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("password must be at least 8 characters".to_string()), payload: None }),
                (len, _, _, _) if len > 32 => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("password cannot exceed 32 characters".to_string()), payload: None }),
                (_, false, _, _) => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("password must contain uppercase letters".to_string()), payload: None }),
                (_, _, false, _) => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("password must contain lowercase letters".to_string()), payload: None }),
                (_, _, _, false) => return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("password must contain numbers".to_string()), payload: None }),
                _ => ()
            };
        }
    }

    // 加密密码并处理可能的错误
    let hashed_passwd = match hash_passwd(&account_request.password.unwrap()) {
        Ok(hashed) => hashed,
        Err(err) => {
            log::error!("failed to hash password: {:?}", err);
            return build_rpc_response(RpcResult::<i64> { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some("Password hashing failed".to_string()), payload: None })
        }
    };
    account_request.password = Some(hashed_passwd);
    account_request.role = Some(Role::User.to_string());

    // 检查邮箱是否已存在
    let db_conn = G_DB.get().unwrap();
    let email = account_request.email.clone().unwrap();
    match Account::count_by_email(db_conn, &email).await {
        Ok(count) => {
            if count > 0 {
                return build_rpc_response(RpcResult::<i64> { code: StatusCode::BAD_REQUEST, msg: Some("email already exists".to_string()), payload: None });
            }
        },
        Err(err) => {
            // 查询数据库出错
            log::error!("failed to fetch account by email: {:?}", err);
            return build_rpc_response(RpcResult::<i64> { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some("Database error checking email".to_string()), payload: None });
        }
    }

    // 写数据并返回
    // 把 account_request 转为 AccountDTO
    let account_dto = AccountDTO {
        email: account_request.email.clone(),
        password: account_request.password.clone(),
        role: account_request.role.clone(),
        api_key: None, email_confirmed_at: None, last_sign_in_at: None, gmt_create: None, gmt_update: None,
    };
    match Account::create(db_conn, &account_dto).await {
        Ok(account_id) => build_rpc_response(RpcResult::<i64> { code: StatusCode::OK, msg: None, payload: Some(account_id) }),
        Err(err) => build_rpc_response(RpcResult::<i64> { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some(err.to_string()), payload: None })
    }
}


// 为用户生成 api_key
#[post("/account/api-key.json")]
async fn generate_api_key(token: JwtToken) -> impl Responder {
    let account_id = token.sub;
    let db_conn = G_DB.get().unwrap();
    
    // 检查用户是否存在
    match Account::fetch_by_id(db_conn, account_id).await {
        Ok(_) => {
            let api_key = generate_secure_api_key(account_id);
            let update_dto = AccountDTO { api_key: Some((&api_key).to_string()) ,
                email: None, password: None, role: None, email_confirmed_at: None, last_sign_in_at: None, gmt_create: None, gmt_update: None
            };
            match Account::update(db_conn, account_id, &update_dto).await {
                Ok(_) => {
                    let token_value = serde_json::json!({"api_key": api_key});
                    build_rpc_response(RpcResult { code: StatusCode::OK, msg: None, payload: Some(token_value) })
                },
                Err(err) => build_rpc_response(RpcResult { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some(format!("update api_key error, {} {}", account_id, err)), payload: None })
            }
        },
        Err(err) => build_rpc_response(RpcResult { code: StatusCode::INTERNAL_SERVER_ERROR, msg: Some(format!("account fetch error: {}", err)), payload: None })
    }
}


lazy_static! {
    // 定义邮箱验证的正则表达式
    static ref EMAIL_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountRequest {
    pub id: Option<i64>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
}