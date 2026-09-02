//! 服务器端处理逻辑。
//!
//! 采用「每连接一个线程」的同步并发模型：
//!
//! 1. 主线程负责 `accept` 新连接；
//! 2. 每接入一个客户端就 `spawn` 一个新线程专门服务它；
//! 3. 所有线程通过 [`std::sync::Arc`] 共享同一个 [`Store`]，
//!    再由 `Store` 内部的互斥锁保证数据访问的并发安全。
//!
//! 这种模型零额外依赖、语义清晰，非常适合课设的基础要求；
//! 若要升级为异步，可把 `TcpListener` 与 `handle_client` 换成 `tokio` 版本。

use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::persistence::Persistence;
use crate::protocol::{read_raw_line, write_message, Request, Response, StatusInfo};
use crate::store::Store;

/// 服务器配置。
pub struct Config {
    /// 监听地址，如 `127.0.0.1:7878`。
    pub addr: String,
    /// 数据文件路径，如 `data/kv.json`。
    pub data_file: String,
}

impl Config {
    /// 用监听地址与数据文件路径构造配置。
    pub fn new(addr: String, data_file: String) -> Self {
        Config { addr, data_file }
    }
}

/// 服务器在多个客户端线程之间共享的状态。
///
/// 通过 `Arc<Store>` 共享存储（所有权共享），`Persistence` 因内部只有
/// 一条路径而直接克隆即可。
#[derive(Clone)]
struct ServerState {
    store: Arc<Store>,
    persistence: Persistence,
    addr: String,
    /// 串行化「快照 + 写盘」的锁。用 `Arc` 包裹是因为 `ServerState`
    /// 会为每个连接克隆一份，所有克隆必须共享**同一把**锁。
    persist_lock: Arc<Mutex<()>>,
}

/// 启动服务器并进入监听循环（阻塞，直到进程被终止）。
///
/// 流程建议：
/// 1. `Persistence::new(...).load()?` 恢复已有数据；
/// 2. `Arc::new(Store::new())` 并 `store.load(initial)` 载入数据；
/// 3. `TcpListener::bind(&config.addr)` 绑定监听地址；
/// 4. 打印监听地址、数据文件路径、已恢复的键值对数量（课设演示要求）；
/// 5. `listener.incoming()` 循环接受连接，每个连接 `thread::spawn` 处理。
///
pub fn run(config: Config) -> Result<()> {
    // 1. 启动时先恢复已有数据（课设要求：重启后数据仍在）。
    let persistence = Persistence::new(&config.data_file);
    let initial = persistence.load()?;

    // 2. 创建共享存储，并载入恢复出的数据。
    let store = Arc::new(Store::new());
    store.load(initial);

    // 3. 绑定监听地址。
    let listener = TcpListener::bind(&config.addr)?;

    // 4. 打印启动信息（课设演示要求：显示监听地址与运行状态）。
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

    // 5. 主循环：不断接受新连接，为每个连接派发一个线程。
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                std::thread::spawn(move || {
                    // 单个客户端的错误不应拖垮整个服务器，只记录日志。
                    if let Err(e) = handle_client(stream, state) {
                        eprintln!("[服务器] 客户端连接异常: {}", e);
                    }
                });
            }
            Err(e) => {
                // 接受连接失败（如系统资源不足），打印后继续等待下一个。
                eprintln!("[服务器] 接受连接失败: {}", e);
            }
        }
    }

    Ok(())
}

/// 服务单个客户端连接：循环「读请求 → 执行 → 写响应」，直到连接断开。
///
/// 建议步骤：
/// 1. `client_connected()` 连接计数 +1，并用 RAII 守卫保证退出时 -1；
/// 2. `stream.try_clone()` 得到两个 handle，一个包成 `BufReader` 读，一个写；
/// 3. 循环读一行请求，解析失败时回错误响应而非断开连接；
/// 4. 解析成功则交给 [`handle_request`] 执行，把响应写回。
///
fn handle_client(stream: TcpStream, state: ServerState) -> Result<()> {
    // 记录客户端地址，便于日志与演示。
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "未知".to_string());

    // 连接数 +1；用 RAII 守卫保证函数结束（无论正常还是出错）时自动 -1。
    state.store.client_connected();
    let _guard = ClientGuard {
        store: Arc::clone(&state.store),
    };

    println!("[服务器] 客户端 {} 已连接", peer);

    // 克隆出两个 handle：一个用于读（包装成 BufReader 按行读），一个用于写。
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    loop {
        // 读取一行请求；返回 None 表示客户端已断开。
        let Some(line) = read_raw_line(&mut reader)? else {
            break;
        };
        if line.trim().is_empty() {
            continue; // 跳过空行
        }

        // 解析请求；解析失败时回一个错误响应而不是断开连接，
        // 这样「格式错误的命令」只会得到提示，不会让系统崩溃。
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => handle_request(&state, request),
            Err(e) => Response::err(format!("无法解析请求: {}", e)),
        };

        write_message(&mut writer, &response)?;
    }

    println!("[服务器] 客户端 {} 已断开", peer);
    Ok(())
}

/// 根据请求类型执行对应操作，返回响应。
///
/// 各命令要点（写操作后记得调用 [`persist`] 落盘）：
/// - `Set`：写内存 + 持久化，返回成功；
/// - `Get`：查不到时返回 `Response::err("键 ... 不存在")`；
/// - `Del`：删到则持久化并返回成功，否则返回「不存在」错误；
/// - `List`：返回所有键；
/// - `Status`：组装 [`crate::protocol::StatusInfo`]（键数、连接数、地址、数据文件路径）。
///
fn handle_request(state: &ServerState, request: Request) -> Response {
    match request {
        Request::Set { key, value } => {
            // 写入内存后立即持久化，保证重启后数据不丢。
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

/// 把当前内存状态持久化到磁盘。
///
/// 必须加锁串行化，否则并发写盘会出两类问题：
/// 1. 多个线程同时写同一个临时文件、又同时 `rename`，其中一个会发现
///    临时文件已被移走，报「No such file or directory」；
/// 2. 「丢失更新」：旧快照的写盘若落在新快照之后，会用旧数据覆盖新数据。
/// 锁住「快照 + 写盘」后，后一次写盘必然看到前一次的所有变更。
///
fn persist(state: &ServerState) -> Result<()> {
    let _guard = state.persist_lock.lock().unwrap();
    let snapshot = state.store.snapshot();
    state.persistence.save(&snapshot)
}

/// RAII 守卫：在连接结束时自动把客户端计数减一，
/// 即使线程因 panic 或错误提前返回也不会漏减。
struct ClientGuard {
    store: Arc<Store>,
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.store.client_disconnected();
    }
}
