//! 命令模型与网络协议。
//!
//! 客户端与服务器之间通过 TCP 传输 **换行分隔的 JSON 文本**（NDJSON）：
//! 每一条消息是一行完整的 JSON，以 `\n` 结尾。选择这种格式的原因：
//!
//! 1. TCP 是字节流，没有天然的消息边界，用换行符可以方便地切分出「一条完整消息」；
//! 2. JSON 可读性强，答辩演示与调试时可以直接看懂传输的内容。
//!
//! 命令模型用**带标签的枚举**表达（Rust 的枚举 + 模式匹配是课设明确要求
//! 掌握的语言特性），`serde` 会把枚举序列化成如下的 JSON 结构：
//!
//! ```text
//! {"type":"set","payload":{"key":"课程名称","value":"Rust程序设计"}}
//! {"type":"get","payload":{"key":"课程名称"}}
//! {"type":"list"}
//! ```

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// 客户端发送给服务器的请求。
///
/// 不同的命令携带的字段各不相同（有的带 `key`，有的带 `value`），
/// 所以用一个带标签的枚举（tagged enum）来表示。`#[serde(...)]` 属性
/// 指定了序列化格式：`type` 字段存放变体名，`payload` 字段存放结构体内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Request {
    /// 写入或覆盖一个键值对。
    Set {
        /// 键
        key: String,
        /// 值
        value: String,
    },
    /// 查询某个键对应的值。
    Get {
        /// 键
        key: String,
    },
    /// 删除某个键。
    Del {
        /// 键
        key: String,
    },
    /// 列出所有已存在的键。
    List,
    /// 查询服务器运行状态（数据量、连接数等）。
    Status,
}

/// 服务器返回给客户端的响应。
///
/// 所有响应都带一个 `ok` 布尔字段表示成功与否：
/// - 成功时 `ok == true`，携带具体结果（`value` / `keys` / `status` 之一）；
/// - 失败时 `ok == false`，`error` 字段给出人类可读的错误信息。
///
/// `#[serde(skip_serializing_if = "Option::is_none")]` 表示当字段为 `None`
/// 时不出现在 JSON 中，让报文更精简、更易读。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// 是否成功。
    pub ok: bool,
    /// 失败原因（仅当 `ok == false` 时有意义）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 查询结果（`Get` 成功时返回）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// 键列表（`List` 成功时返回）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
    /// 状态信息（`Status` 成功时返回）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusInfo>,
}

impl Response {
    /// 构造一个不带额外结果的成功响应。
    pub fn ok() -> Self {
        Response {
            ok: true,
            error: None,
            value: None,
            keys: None,
            status: None,
        }
    }

    /// 构造一个失败的响应，附带错误信息。
    pub fn err(msg: impl Into<String>) -> Self {
        Response {
            ok: false,
            error: Some(msg.into()),
            value: None,
            keys: None,
            status: None,
        }
    }

    /// 附带查询结果。
    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    /// 附带键列表。
    pub fn with_keys(mut self, keys: Vec<String>) -> Self {
        self.keys = Some(keys);
        self
    }

    /// 附带状态信息。
    pub fn with_status(mut self, status: StatusInfo) -> Self {
        self.status = Some(status);
        self
    }
}

/// 服务器运行状态信息（`Status` 命令的响应内容）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusInfo {
    /// 当前存储的键值对数量。
    pub key_count: usize,
    /// 当前活跃的客户端连接数。
    pub client_count: usize,
    /// 服务器监听的地址。
    pub listen_addr: String,
    /// 数据文件的路径。
    pub data_file: String,
}

/// 从缓冲读流中读取一行（去掉行尾的换行符与回车符）。
///
/// 返回 `None` 表示对端已关闭连接（读到 EOF）。返回值为
/// `Result<Option<String>>`：`Err` 表示 I/O 出错，`Ok(None)` 表示正常关闭。
///
/// 提示：用 [`BufRead::read_line`] 读入一行；返回 0 字节即 EOF；
/// 记得去掉行尾的 `\n` 和 `\r`（兼容 Windows 的 CRLF 换行）。
///
/// 【待实现】
pub fn read_raw_line<R: BufRead>(reader: &mut R) -> Result<Option<String>> {
    todo!("实现 read_raw_line：按行读取，EOF 返回 Ok(None)，去掉行尾换行符")
}

/// 读取一行并反序列化为指定类型 `T`。
///
/// `None` 表示对端关闭连接；解析失败会返回 `Err`。
/// 可复用 [`read_raw_line`] 拿到一行文本，再用 `serde_json::from_str` 解析。
///
/// 【待实现】
pub fn read_message<R: BufRead, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<Option<T>> {
    todo!("实现 read_message：读取一行并反序列化为 T")
}

/// 把消息序列化成一行 JSON 写入流中，并在末尾补换行作为消息边界。
///
/// 函数对消息类型 `T` 泛型，因此既可以写 [`Request`] 也可以写 [`Response`]。
/// 提示：`serde_json::to_string` 序列化，`writeln!` 补换行，最后 `flush`。
///
/// 【待实现】
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> Result<()> {
    todo!("实现 write_message：序列化并写入一行 JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证请求枚举能正确序列化与反序列化（往返一致）。
    #[test]
    fn request_roundtrip() {
        let req = Request::Set {
            key: "课程名称".into(),
            value: "Rust程序设计".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        // 断言 JSON 结构符合预期（type + payload）。
        assert_eq!(
            json,
            r#"{"type":"set","payload":{"key":"课程名称","value":"Rust程序设计"}}"#
        );
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }

    /// 无参命令（如 List）序列化后不应带 payload。
    #[test]
    fn unit_variant_roundtrip() {
        let req = Request::List;
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"type":"list"}"#);
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Request::List);
    }

    /// 响应中为 `None` 的字段不应出现在 JSON 里。
    #[test]
    fn response_skips_none_fields() {
        let resp = Response::ok().with_value("你好".into());
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(json, r#"{"ok":true,"value":"你好"}"#);
    }

    /// 验证消息的「写一行 → 读一行」往返。
    #[test]
    fn message_roundtrip_over_buffer() {
        let mut buf = Vec::new();
        let req = Request::Get { key: "k".into() };
        write_message(&mut buf, &req).unwrap();

        let mut reader = std::io::BufReader::new(&buf[..]);
        let back: Request = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(back, req);
    }
}
