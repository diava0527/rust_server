//! 客户端可执行文件入口。
//!
//! 运行方式：
//! ```text
//! cargo run --bin client
//! cargo run --bin client -- --addr 127.0.0.1:7878
//! ```

use clap::Parser;

use kvstore::client;

/// 命令行参数定义。
#[derive(Parser, Debug)]
#[command(name = "kvclient", version, about = "可持久化网络键值存储系统 —— 命令行客户端")]
struct Args {
    /// 服务器地址（默认 127.0.0.1:7878）
    #[arg(short, long, default_value = "127.0.0.1:7878")]
    addr: String,
}

fn main() {
    // 解析命令行参数。
    let args = Args::parse();

    // 启动交互式客户端；出错时打印并以非零码退出。
    if let Err(e) = client::run(&args.addr) {
        eprintln!("客户端退出: {}", e);
        std::process::exit(1);
    }
}
