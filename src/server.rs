//! 服务器端：监听连接、并发处理客户端请求。

use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::persistence::Persistence;
use crate::protocol::{read_raw_line, write_message, Request, Response, StatusInfo};
use crate::store::Store;

pub struct Config {
    pub addr: String,
    pub data_file: String,
}

impl Config {
    pub fn new(addr: String, data_file: String) -> Self {
        Config { addr, data_file }
    }
}

/// 多个客户端线程共享的状态。
#[derive(Clone)]
struct ServerState {
    store: Arc<Store>,
    persistence: Persistence,
    addr: String,
    /// 串行化「快照 + 写盘」的锁。
    persist_lock: Arc<Mutex<()>>,
}

pub fn run(config: Config) -> Result<()> {
    // 启动时先恢复之前的数据。
    let persistence = Persistence::new(&config.data_file);
    let initial = persistence.load()?;

    let store = Arc::new(Store::new());
    store.load(initial);

    let listener = TcpListener::bind(&config.addr)?;

    println!("[服务器] 已启动，监听地址: {}", config.addr);
    println!("[服务器] 数据文件: {}", config.data_file);
    println!("[服务器] 已恢复 {} 个键值对", store.len());
    println!("[服务器] 等待客户端连接...");

    let state = ServerState {
        store,
        persistence,
        addr: config.addr,
        persist_lock: Arc::new(Mutex::new(())),
    };

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream, state) {
                        eprintln!("[服务器] 客户端连接异常: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("[服务器] 接受连接失败: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(stream: TcpStream, state: ServerState) -> Result<()> {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "未知".to_string());

    state.store.client_connected();
    // ClientGuard 在函数结束时（包括提前出错）会自动把计数减一。
    let _guard = ClientGuard {
        store: Arc::clone(&state.store),
    };

    println!("[服务器] 客户端 {} 已连接", peer);

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    loop {
        let Some(line) = read_raw_line(&mut reader)? else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }

        // 解析失败时回错误响应，而不是直接断开连接。
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => handle_request(&state, request),
            Err(e) => Response::err(format!("无法解析请求: {}", e)),
        };

        write_message(&mut writer, &response)?;
    }

    println!("[服务器] 客户端 {} 已断开", peer);
    Ok(())
}

fn handle_request(state: &ServerState, request: Request) -> Response {
    match request {
        Request::Set { key, value } => {
            state.store.set(&key, &value);
            match persist(state) {
                Ok(()) => Response::ok().with_value(value),
                Err(e) => Response::err(format!("数据已写入内存，但持久化失败: {}", e)),
            }
        }
        Request::Get { key } => match state.store.get(&key) {
            Some(value) => Response::ok().with_value(value),
            None => Response::err(format!("键 \"{}\" 不存在", key)),
        },
        Request::Del { key } => match state.store.del(&key) {
            Some(_) => match persist(state) {
                Ok(()) => Response::ok(),
                Err(e) => Response::err(format!("数据已删除，但持久化失败: {}", e)),
            },
            None => Response::err(format!("键 \"{}\" 不存在", key)),
        },
        Request::List => Response::ok().with_keys(state.store.keys()),
        Request::Status => {
            let status = StatusInfo {
                key_count: state.store.len(),
                client_count: state.store.client_count(),
                listen_addr: state.addr.clone(),
                data_file: state.persistence.path().display().to_string(),
            };
            Response::ok().with_status(status)
        }
    }
}

// 加锁串行化，避免并发写盘时临时文件冲突或旧快照覆盖新快照。
fn persist(state: &ServerState) -> Result<()> {
    let _guard = state.persist_lock.lock().unwrap();
    let snapshot = state.store.snapshot();
    state.persistence.save(&snapshot)
}

/// 连接结束时自动把计数减一。
struct ClientGuard {
    store: Arc<Store>,
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.store.client_disconnected();
    }
}
