//! 服务器可执行文件入口。
//!
//! 运行方式：
//! ```text
//! cargo run --bin server
//! cargo run --bin server -- --addr 127.0.0.1:7878 --data-file data/kv.json
//! ```

use clap::Parser;

use kvstore::server;

/// 命令行参数定义（由 `clap` 的 derive 宏从结构体自动生成）。
#[derive(Parser, Debug)]
#[command(name = "kvserver", version, about = "可持久化网络键值存储系统 —— 服务器端")]
struct Args {
    /// 监听地址（默认 127.0.0.1:7878）
    #[arg(short, long, default_value = "127.0.0.1:7878")]
    addr: String,

    /// 数据文件路径（默认 data/kv.json）
    #[arg(short, long, default_value = "data/kv.json")]
    data_file: String,
}

fn main() {
    // 解析命令行参数。
    let args = Args::parse();

    // 启动服务器；启动失败时打印错误并以非零码退出。
    if let Err(e) = server::run(server::Config::new(args.addr, args.data_file)) {
        eprintln!("服务器启动失败: {}", e);
        std::process::exit(1);
    }
}
