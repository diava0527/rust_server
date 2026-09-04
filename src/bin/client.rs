use clap::Parser;

use kvstore::client;

#[derive(Parser, Debug)]
#[command(name = "kvclient", version, about = "可持久化网络键值存储系统 —— 命令行客户端")]
struct Args {
    /// 服务器地址
    #[arg(short, long, default_value = "127.0.0.1:7878")]
    addr: String,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = client::run(&args.addr) {
        eprintln!("客户端退出: {}", e);
        std::process::exit(1);
    }
}
