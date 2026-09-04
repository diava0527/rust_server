use thiserror::Error;

/// 整个系统统一使用的错误类型。
#[derive(Debug, Error)]
pub enum KvError {
    /// 底层 I/O 错误（网络、文件读写等）。
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化 / 反序列化错误。
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    /// 数据文件损坏或格式非法。
    #[error("数据文件损坏或格式非法: {0}")]
    CorruptedData(String),

    /// 服务器返回了无法解析 / 非法的响应。
    #[error("服务器响应非法: {0}")]
    InvalidResponse(String),

    /// 连接被对端意外关闭。
    #[error("连接已关闭")]
    ConnectionClosed,
}

/// 项目通用 Result 别名，错误类型固定为 KvError。
pub type Result<T> = std::result::Result<T, KvError>;
