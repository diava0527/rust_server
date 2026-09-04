//! 通信协议：请求/响应类型与消息读写。

use std::io::{BufRead, Write};
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// 客户端发送给服务器的请求，用带标签的枚举表示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Request {
    /// 写入或覆盖一个键值对。
    Set {
        key: String,
        value: String,
    },
    /// 查询某个键对应的值。
    Get {
        key: String,
    },
    /// 删除某个键。
    Del {
        key: String,
    },
    /// 列出所有已存在的键。
    List,
    /// 查询服务器运行状态。
    Status,
}

/// 服务器返回给客户端的响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// 是否成功。
    pub ok: bool,
    /// 失败原因。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 查询结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// 键列表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
    /// 状态信息。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusInfo>,
}

impl Response {
    pub fn ok() -> Self {
        Response {
            ok: true,
            error: None,
            value: None,
            keys: None,
            status: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Response {
            ok: false,
            error: Some(msg.into()),
            value: None,
            keys: None,
            status: None,
        }
    }

    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_keys(mut self, keys: Vec<String>) -> Self {
        self.keys = Some(keys);
        self
    }

    pub fn with_status(mut self, status: StatusInfo) -> Self {
        self.status = Some(status);
        self
    }
}

/// 服务器运行状态信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusInfo {
    pub key_count: usize,
    pub client_count: usize,
    pub listen_addr: String,
    pub data_file: String,
}

/// 从流里读一行，去掉末尾的换行符。读到 EOF 返回 Ok(None)。
pub fn read_raw_line<R: BufRead>(reader: &mut R) -> Result<Option<String>> {
    let mut line = String::new();
    let bytes = reader.read_line(&mut line)?;
    if bytes == 0 {
        return Ok(None);
    }
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(Some(line))
}

/// 读一行并反序列化成 T。
pub fn read_message<R: BufRead, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<Option<T>> {
    match read_raw_line(reader)? {
        Some(line) => {
            let msg = serde_json::from_str(&line)?;
            Ok(Some(msg))
        }
        None => Ok(None),
    }
}

/// 把消息序列化成一行 JSON 写入，末尾补换行。
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> Result<()> {
    let json = serde_json::to_string(msg)?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = Request::Set {
            key: "课程名称".into(),
            value: "Rust程序设计".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(
            json,
            r#"{"type":"set","payload":{"key":"课程名称","value":"Rust程序设计"}}"#
        );
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn unit_variant_roundtrip() {
        let req = Request::List;
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"type":"list"}"#);
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Request::List);
    }

    #[test]
    fn response_skips_none_fields() {
        let resp = Response::ok().with_value("你好".into());
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(json, r#"{"ok":true,"value":"你好"}"#);
    }

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
