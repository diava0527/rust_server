use std::io::BufReader;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use kvstore::protocol::{read_message, write_message, Request, Response};

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

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

fn spawn_server(port: u16, data_file: &str) -> (Child, String) {
    let bin = env!("CARGO_BIN_EXE_server");
    let addr = format!("127.0.0.1:{}", port);
    let child = Command::new(bin)
        .arg("--addr")
        .arg(&addr)
        .arg("--data-file")
        .arg(data_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("启动服务器进程失败");
    (child, addr)
}

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

fn request(addr: &str, req: &Request) -> Response {
    let mut stream = TcpStream::connect(addr).unwrap();
    write_message(&mut stream, req).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    read_message::<_, Response>(&mut reader).unwrap().unwrap()
}

fn cleanup(server: &mut Child, data_file: &str) {
    let _ = server.kill();
    let _ = server.wait();
    let _ = std::fs::remove_file(data_file);
}

#[test]
fn end_to_end_basic_flow() {
    let port = free_port();
    let data_file = temp_data_file("basic");
    let (mut server, addr) = spawn_server(port, &data_file);
    wait_ready(&addr);

    let resp = request(
        &addr,
        &Request::Set {
            key: "课程名称".into(),
            value: "Rust程序设计".into(),
        },
    );
    assert!(resp.ok, "写入应成功: {:?}", resp);

    let resp = request(
        &addr,
        &Request::Get {
            key: "课程名称".into(),
        },
    );
    assert!(resp.ok);
    assert_eq!(resp.value.as_deref(), Some("Rust程序设计"));

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

    let resp = request(
        &addr,
        &Request::Del {
            key: "课程名称".into(),
        },
    );
    assert!(resp.ok, "删除应成功: {:?}", resp);

    let resp = request(
        &addr,
        &Request::Get {
            key: "课程名称".into(),
        },
    );
    assert!(!resp.ok, "查询不存在的键应失败");
    assert!(resp.error.is_some(), "应给出明确的错误提示");

    let resp = request(&addr, &Request::List);
    assert!(resp.ok);
    let resp = request(&addr, &Request::Status);
    assert!(resp.ok);
    assert!(resp.status.is_some());

    cleanup(&mut server, &data_file);
}

#[test]
fn concurrent_clients() {
    let port = free_port();
    let data_file = temp_data_file("concurrent");
    let (mut server, addr) = spawn_server(port, &data_file);
    wait_ready(&addr);

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

    let resp = request(&addr, &Request::List);
    assert!(resp.ok);
    let keys = resp.keys.expect("应返回键列表");
    assert_eq!(keys.len(), 8, "应恰好有 8 个键，实际是 {:?}", keys);

    cleanup(&mut server, &data_file);
}

#[test]
fn persistence_across_restart() {
    let data_file = temp_data_file("restart");

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

    let _ = server.kill();
    let _ = server.wait();

    let (mut server2, addr2) = spawn_server(free_port(), &data_file);
    wait_ready(&addr2);

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
