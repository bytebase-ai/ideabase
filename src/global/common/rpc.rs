use http::StatusCode;

/// 标准RPC响应结构体
#[derive(Debug, Clone)]
pub struct RpcResult<T: serde::Serialize> {
    /// HTTP状态码，与http::StatusCode一致
    pub code: StatusCode,
    /// 人类可读的消息
    pub msg: Option<String>,
    /// 响应数据负载
    pub payload: Option<T>,
}