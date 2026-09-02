//! 统一的错误类型定义。
//!
//! 使用 [`thiserror`] 的派生宏，为每个错误变体自动生成 `Display` 和
//! `std::error::Error` 实现。这样错误既能被 `?` 运算符方便地向上传播，
//! 又能在最外层被转换成人类可读的提示信息（对应课设要求的「识别并处理
//! 文件、网络和命令输入中的异常」）。

use thiserror::Error;

/// 整个系统使用的统一错误类型。
///
/// 设计要点：
/// - 网络读写错误与文件读写错误统一归为 [`KvError::Io`]（通过 `#[from]` 自动转换）；
/// - 序列化错误归为 [`KvError::Serde`]；
/// - 数据文件损坏是**业务层**错误，必须与普通 I/O 错误区分开，
///   以便在启动恢复时给出「文件损坏、拒绝静默清空」的明确提示。
#[derive(Debug, Error)]
pub enum KvError {
    /// 底层 I/O 错误（网络、文件读写等）。
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化 / 反序列化错误。
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    /// 数据文件损坏或格式非法（启动恢复时抛出，绝不静默清空数据）。
    #[error("数据文件损坏或格式非法: {0}")]
    CorruptedData(String),

    /// 服务器返回了无法解析 / 非法的响应。
    #[error("服务器响应非法: {0}")]
    InvalidResponse(String),

    /// 连接被对端意外关闭。
    #[error("连接已关闭")]
    ConnectionClosed,
}

/// 项目的通用 `Result` 别名，错误类型固定为 [`KvError`]。
pub type Result<T> = std::result::Result<T, KvError>;
