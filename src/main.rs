mod controller;
pub mod global;
pub mod service;

use crate::global::common::log::init_tk_log;
use crate::global::common::yaml::{load_global_config, GlobalConfig};
use crate::global::db::core::DBConn;

/// 全局配置静态实例
/// 使用 lazy_static 宏在首次访问时初始化全局配置
#[macro_use] extern crate lazy_static;
lazy_static! {
    /// 全局环境配置变量
    /// 通过 `load_global_config()` 函数加载配置文件内容
    pub static ref G_ENV: GlobalConfig = load_global_config();
}

/// 全局数据库连接池实例
/// 使用 OnceCell 实现延迟初始化的全局单例模式
pub static G_DB: once_cell::sync::OnceCell<DBConn> = once_cell::sync::OnceCell::new();

/// 应用程序主入口点
/// 
/// # 功能说明
/// - 初始化日志系统
/// - 初始化数据库连接池
/// - 启动 HTTP 服务器
/// 
/// # 返回值
/// - `std::io::Result<()>` - 启动成功返回 Ok，失败返回相应的错误信息
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志系统
    init_tk_log();

    // 数据源初始化 - 从环境变量获取 MySQL 连接字符串
    if let Ok(mysql_url) = std::env::var("MYSQL_URL") {
        // 建立数据库连接池
        let db_conn = init_datasource_conn(&mysql_url).await.expect("datasource init error");
        // 设置全局数据库连接池
        G_DB.set(db_conn).unwrap();
    } else {
        // 环境变量未设置，记录错误并退出程序
        log::error!("MYSQL_URL not set");
        std::process::exit(-11);
    }

    // 创建 HTTP 服务器实例
    let http_server = actix_web::HttpServer::new(|| {
        actix_web::App::new()
            // 添加日志中间件
            .wrap(actix_web::middleware::Logger::default())
            // 添加 CORS 跨域支持
            .wrap(controller::cors())
            // 注册路由控制器
            .configure(controller::register_routes)
    });
    
    // 记录服务器启动日志
    log::info!("IDEA-BASE starting at http://0.0.0.0:8080");
    // 启动服务器，绑定地址并运行
    http_server.workers(4).bind(("0.0.0.0", 8080))?.run().await
}

/// 初始化数据库连接池
/// 
/// # 参数
/// - `url`: 数据库连接字符串
/// 
/// # 返回值
/// - `Result<DBConn, sqlx::Error>` - 成功返回数据库连接池实例，失败返回错误信息
async fn init_datasource_conn(url: &str) -> Result<DBConn, sqlx::Error> {
    DBConn::new(url).await
}