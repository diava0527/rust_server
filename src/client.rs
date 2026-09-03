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

use crate::protocol::{read_message, write_message, Request, Response};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use crate::error::Result;

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
pub fn run(addr: &str) -> Result<()> {
    let stream = TcpStream::connect(addr)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut stdout = io::stdout();

    loop {
        print!("kv> ");
        stdout.flush()?;

        let mut input = String::new();
        if stdin_lock.read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match parse_command(input) {
            Ok(Command::Request(req)) => {
                if let Err(e) = write_message(&mut writer, &req) {
                    eprintln!("发送请求失败: {}", e);
                    continue;
                }
                match read_message::<_, Response>(&mut reader) {
                    Ok(Some(resp)) => print_response(&resp),
                    Ok(None) => {
                        println!("服务器关闭连接");
                        break;
                    }
                    Err(e) => {
                        eprintln!("接收响应失败: {}", e);
                        continue;
                    }
                }
            }
            Ok(Command::Quit) => break,
            Ok(Command::Help) => print_help(),
            Err(e) => eprintln!("{}", e),
        }
    }
    Ok(())
}

/// 把一行用户输入解析成 [`Command`]。
///
/// 解析规则：命令名是第一个空白之前的 token；其余部分按命令类型继续拆分。
/// 返回 `Err(String)` 表示无法识别或参数缺失，错误信息直接面向用户。
///
/// 提示：用 `line.find(char::is_whitespace)` 拆出命令名与剩余参数；
/// `set` 需要把剩余参数再拆成「键 + 值」（值可含空格，见 [`split_key_value`]）。
fn parse_command(line: &str) -> std::result::Result<Command, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("空命令".to_string());
    }

    let first_space = trimmed.find(char::is_whitespace);
    let (cmd, rest) = if let Some(pos) = first_space {
        (trimmed[..pos].to_lowercase(), trimmed[pos..].trim_start())
    } else {
        (trimmed.to_lowercase(), "")
    };

    match cmd.as_str() {
        "set" => {
            let (key, value) = split_key_value(rest)
                .ok_or_else(|| "缺少键或值".to_string())?;
            Ok(Command::Request(Request::Set { key, value }))
        }
        "get" => {
            if rest.is_empty() {
                return Err("缺少键".to_string());
            }
            Ok(Command::Request(Request::Get { key: rest.to_string() }))
        }
        "del" => {
            if rest.is_empty() {
                return Err("缺少键".to_string());
            }
            Ok(Command::Request(Request::Del { key: rest.to_string() }))
        }
        "list" => Ok(Command::Request(Request::List)),
        "status" => Ok(Command::Request(Request::Status)),
        "quit" | "exit" => Ok(Command::Quit),
        "help" => Ok(Command::Help),
        _ => Err(format!("未知命令: {}", cmd)),
    }
}

/// 把 `key value` 切分成键和值；值允许包含空格。
///
/// 例如 `"课程名称 Rust 程序设计"` → `("课程名称", "Rust 程序设计")`。
/// 键或值为空时返回 `None`。
fn split_key_value(rest: &str) -> Option<(String, String)> {
    let pos = rest.find(char::is_whitespace)?;
    let key = rest[..pos].trim();
    if key.is_empty() {
        return None;
    }
    let value = rest[pos..].trim_start();
    if value.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
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
fn print_response(response: &Response) {
    if response.ok {
        if let Some(value) = &response.value {
            println!("{}", value);
        } else if let Some(keys) = &response.keys {
            if keys.is_empty() {
                println!("(空)");
            } else {
                println!("keys: {:?}", keys);
            }
        } else if let Some(status) = &response.status {
            println!("键数量: {}", status.key_count);
            println!("客户端连接数: {}", status.client_count);
            println!("监听地址: {}", status.listen_addr);
            println!("数据文件: {}", status.data_file);
        } else {
            println!("OK");
        }
    } else {
        if let Some(err) = &response.error {
            eprintln!("错误: {}", err);
        } else {
            eprintln!("操作失败");
        }
    }
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
