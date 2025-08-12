pub mod rest_controller;
pub mod account_controller;

use actix_web::{get, http::StatusCode, web, HttpResponse, HttpResponseBuilder, Responder};
use crate::global::common::rpc::RpcResult;

// 快速返回结果
pub fn build_rpc_response<T: serde::Serialize>(rpc_result: RpcResult<T>) -> impl Responder {
    let status_code = rpc_result.code;
    let err_msg = rpc_result.msg;
    let payload = rpc_result.payload;
    if status_code == StatusCode::OK {
        let json_payload = match payload {
            Some(value) => serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize payload"})),
            None => serde_json::json!({}),
        };
        HttpResponse::Ok().json(json_payload)
    } else {
        HttpResponseBuilder::new(status_code).json(serde_json::json!({"err_msg": err_msg}))
    }
}

pub fn cors() -> actix_cors::Cors {
    actix_cors::Cors::default()
        .allowed_origin("https://ideabase.io")
        .allowed_methods(vec!["*"])
        .allowed_headers(vec!["content-type"])
        .supports_credentials()
        .max_age(3600)
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    // health
    cfg.service(health);
    // module scope
    cfg.service(web::scope("/api/v1")
        .service(account_controller::scope())
        .service(rest_controller::scope())
    ;
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json("I'm OK!")
}

