//! 集成测试：验证「服务器二进制 ↔ 客户端协议」之间的端到端交互。
//!
//! 说明：这里的「客户端」直接用 `kvstore::protocol` 与真实启动的服务器进程
//! 通信，覆盖了 TCP 传输、JSON 序列化、请求分发、并发访问与持久化的完整链路。
//!
//! 通过 `env!("CARGO_BIN_EXE_server")` 拿到 Cargo 已编译好的服务器二进制路径，
//! 用子进程方式启动它，从而测试**真实的网络路径**（而非库内的函数调用）。

use std::io::BufReader;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use kvstore::protocol::{read_message, write_message, Request, Response};

/// 找一个当前可用的临时端口号。
///
/// 原理：先绑定到端口 0（由操作系统分配一个空闲端口），读出端口号后
/// 立刻释放监听，再把这个端口交给服务器进程使用。
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// 生成唯一的临时数据文件路径，避免多个测试并行运行时互相覆盖。
fn temp_data_file(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir()
        .join(format!("kvstore_it_{}_{}.json", tag, nanos))
        .to_string_lossy()
        .into_owned()
}

/// 启动服务器二进制，返回子进程句柄和监听地址。
fn spawn_server(port: u16, data_file: &str) -> (Child, String) {
    let bin = env!("CARGO_BIN_EXE_server");
    let addr = format!("127.0.0.1:{}", port);
    let child = Command::new(bin)
        .arg("--addr")
        .arg(&addr)
        .arg("--data-file")
        .arg(data_file)
        // 静默子进程输出，保持 cargo test 输出整洁。
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("启动服务器进程失败");
    (child, addr)
}

/// 等待服务器就绪：反复尝试连接，直到成功或超时。
fn wait_ready(addr: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        assert!(Instant::now() < deadline, "等待服务器就绪超时: {}", addr);
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 建立一条到服务器的连接，发送请求并读取响应（一次请求往返）。
fn request(addr: &str, req: &Request) -> Response {
    let mut stream = TcpStream::connect(addr).unwrap();
    write_message(&mut stream, req).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    read_message::<_, Response>(&mut reader).unwrap().unwrap()
}

/// 结束服务器子进程并清理数据文件。
fn cleanup(server: &mut Child, data_file: &str) {
    let _ = server.kill();
    let _ = server.wait();
    let _ = std::fs::remove_file(data_file);
}

/// 端到端基础流程：写入、查询、覆盖、删除、列表、状态、查询不存在的键。
#[test]
fn end_to_end_basic_flow() {
    let port = free_port();
    let data_file = temp_data_file("basic");
    let (mut server, addr) = spawn_server(port, &data_file);
    wait_ready(&addr);

    // 写入
    let resp = request(
        &addr,
        &Request::Set {
            key: "课程名称".into(),
            value: "Rust程序设计".into(),
        },
    );
    assert!(resp.ok, "写入应成功: {:?}", resp);

    // 查询
    let resp = request(
        &addr,
        &Request::Get {
            key: "课程名称".into(),
        },
    );
    assert!(resp.ok);
    assert_eq!(resp.value.as_deref(), Some("Rust程序设计"));

    // 覆盖：写入同一键的新值
    request(
        &addr,
        &Request::Set {
            key: "课程名称".into(),
            value: "Rust程序设计(第二版)".into(),
        },
    );
    let resp = request(
        &addr,
        &Request::Get {
            key: "课程名称".into(),
        },
    );
    assert_eq!(resp.value.as_deref(), Some("Rust程序设计(第二版)"));

    // 删除
    let resp = request(
        &addr,
        &Request::Del {
            key: "课程名称".into(),
        },
    );
    assert!(resp.ok, "删除应成功: {:?}", resp);

    // 查询不存在的键：应返回失败并带错误信息
    let resp = request(
        &addr,
        &Request::Get {
            key: "课程名称".into(),
        },
    );
    assert!(!resp.ok, "查询不存在的键应失败");
    assert!(resp.error.is_some(), "应给出明确的错误提示");

    // 列表与状态
    let resp = request(&addr, &Request::List);
    assert!(resp.ok);
    let resp = request(&addr, &Request::Status);
    assert!(resp.ok);
    assert!(resp.status.is_some());

    cleanup(&mut server, &data_file);
}

/// 多客户端并发访问：同时发起多个写入，全部成功且数据完整。
#[test]
fn concurrent_clients() {
    let port = free_port();
    let data_file = temp_data_file("concurrent");
    let (mut server, addr) = spawn_server(port, &data_file);
    wait_ready(&addr);

    // 启动 8 个线程，各自写入一个键。
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let addr = addr.clone();
            std::thread::spawn(move || {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                let resp = request(&addr, &Request::Set { key, value });
                assert!(resp.ok, "并发写入应成功: {:?}", resp);
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    // 验证 8 个键全部写入成功。
    let resp = request(&addr, &Request::List);
    assert!(resp.ok);
    let keys = resp.keys.expect("应返回键列表");
    assert_eq!(keys.len(), 8, "应恰好有 8 个键，实际是 {:?}", keys);

    cleanup(&mut server, &data_file);
}

/// 持久化与重启恢复：写入后关闭服务器，重启后数据仍然存在。
#[test]
fn persistence_across_restart() {
    let data_file = temp_data_file("restart");

    // 第一次启动：写入数据。
    let (mut server, addr) = spawn_server(free_port(), &data_file);
    wait_ready(&addr);
    let resp = request(
        &addr,
        &Request::Set {
            key: "课程名称".into(),
            value: "Rust程序设计".into(),
        },
    );
    assert!(resp.ok, "写入应成功: {:?}", resp);

    // 关闭服务器（此时数据已同步写入磁盘）。
    let _ = server.kill();
    let _ = server.wait();

    // 第二次启动：换一个新端口（避免 TIME_WAIT），复用同一数据文件。
    let (mut server2, addr2) = spawn_server(free_port(), &data_file);
    wait_ready(&addr2);

    // 验证数据已被恢复。
    let resp = request(
        &addr2,
        &Request::Get {
            key: "课程名称".into(),
        },
    );
    assert!(resp.ok, "重启后应能查询到数据: {:?}", resp);
    assert_eq!(resp.value.as_deref(), Some("Rust程序设计"));

    cleanup(&mut server2, &data_file);
}
