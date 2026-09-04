//! 命令行客户端。

use crate::protocol::{read_message, write_message, Request, Response};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use crate::error::Result;

/// 客户端本地解析出的命令。
#[derive(Debug)]
enum Command {
    Request(Request),
    Quit,
    Help,
}

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
