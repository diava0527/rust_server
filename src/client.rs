//! 命令行客户端逻辑。
//!
//! 负责：
//! 1. 解析用户在终端输入的命令（如 `set 课程名称 Rust程序设计`）；
//! 2. 与服务器建立一条长连接；
//! 3. 把命令打包成 [`Request`] 发送给服务器；
//! 4. 接收并格式化显示服务器的 [`Response`]。
//!
//! 支持的命令（与服务器协议一一对应）：
//!
//! ```text
//! set <key> <value>   写入 / 覆盖一个键值对（value 可以含空格）
//! get <key>           查询键对应的值
//! del <key>           删除键
//! list                列出所有键
//! status              查看服务器状态
//! help                显示帮助
//! quit / exit         退出客户端
//! ```

use crate::error::Result;
use crate::protocol::{Request, Response};

/// 客户端本地解析出的命令。
#[derive(Debug)]
enum Command {
    /// 一个需要发送给服务器的请求。
    Request(Request),
    /// 退出客户端。
    Quit,
    /// 显示帮助。
    Help,
}

/// 启动交互式客户端（阻塞，直到用户输入 quit/exit 或连接断开）。
///
/// 建议步骤：
/// 1. `TcpStream::connect(addr)` 建立连接，`try_clone` 出读/写两个 handle；
/// 2. 循环：打印 `kv> ` 提示符，`stdin().lock().read_line` 读一行；
/// 3. 交给 [`parse_command`] 解析，`Request` 则 `write_message` 发送，
///    再 `read_message` 读响应并 [`print_response`] 显示；
/// 4. `Quit` 或读到 EOF 则退出。
///
/// 实现时需补 `use std::io::{self, BufRead, BufReader, Write};`、
/// `use std::net::TcpStream;` 与 `use crate::protocol::{read_message, write_message};`。
///
/// 【待实现】
pub fn run(addr: &str) -> Result<()> {
    todo!("实现 run：交互式读取命令、收发消息、显示结果")
}

/// 把一行用户输入解析成 [`Command`]。
///
/// 解析规则：命令名是第一个空白之前的 token；其余部分按命令类型继续拆分。
/// 返回 `Err(String)` 表示无法识别或参数缺失，错误信息直接面向用户。
///
/// 提示：用 `line.find(char::is_whitespace)` 拆出命令名与剩余参数；
/// `set` 需要把剩余参数再拆成「键 + 值」（值可含空格，见 [`split_key_value`]）。
///
/// 【待实现】
fn parse_command(line: &str) -> std::result::Result<Command, String> {
    todo!("实现 parse_command：解析 set/get/del/list/status/quit/help")
}

/// 把 `key value` 切分成键和值；值允许包含空格。
///
/// 例如 `"课程名称 Rust 程序设计"` → `("课程名称", "Rust 程序设计")`。
/// 键或值为空时返回 `None`。
///
/// 【待实现】
fn split_key_value(rest: &str) -> Option<(String, String)> {
    todo!("实现 split_key_value：在第一个空白处切分键与值")
}

/// 打印客户端帮助信息（命令清单）。
fn print_help() {
    println!("可用命令:");
    println!("  set <key> <value>   写入或覆盖键值对");
    println!("  get <key>           查询键对应的值");
    println!("  del <key>           删除键");
    println!("  list                列出所有键");
    println!("  status              查看服务器状态");
    println!("  help                显示本帮助");
    println!("  quit / exit         退出客户端");
}

/// 格式化打印服务器响应。
///
/// 要点：
/// - `ok == true` 时：打印 `value`（查询结果）、`keys`（列表，空则打印"(空)"）、
///   `status`（状态信息各字段）；三者皆无则打印 `OK`（删除成功等）；
/// - `ok == false` 时：打印 `error` 里的错误信息。
///
/// 【待实现】
fn print_response(response: &Response) {
    todo!("实现 print_response：按成功/失败与字段格式化打印")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_set_with_spaces_in_value() {
        let cmd = parse_command("set 课程名称 Rust 程序设计").unwrap();
        match cmd {
            Command::Request(Request::Set { key, value }) => {
                assert_eq!(key, "课程名称");
                assert_eq!(value, "Rust 程序设计");
            }
            other => panic!("应解析为 Set 命令，实际是 {:?}", other),
        }
    }

    #[test]
    fn parse_get_and_del() {
        assert!(matches!(
            parse_command("get 课程名称").unwrap(),
            Command::Request(Request::Get { .. })
        ));
        assert!(matches!(
            parse_command("del 课程名称").unwrap(),
            Command::Request(Request::Del { .. })
        ));
    }

    #[test]
    fn parse_list_status_quit() {
        assert!(matches!(
            parse_command("list").unwrap(),
            Command::Request(Request::List)
        ));
        assert!(matches!(
            parse_command("status").unwrap(),
            Command::Request(Request::Status)
        ));
        assert!(matches!(parse_command("quit").unwrap(), Command::Quit));
        assert!(matches!(parse_command("exit").unwrap(), Command::Quit));
    }

    #[test]
    fn parse_missing_args_errors() {
        assert!(parse_command("set 只有键").is_err());
        assert!(parse_command("get").is_err());
        assert!(parse_command("del").is_err());
    }

    #[test]
    fn parse_unknown_command_errors() {
        assert!(parse_command("foobar").is_err());
    }
}
