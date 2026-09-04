use clap::Parser;

use kvstore::server;

#[derive(Parser, Debug)]
#[command(name = "kvserver", version, about = "可持久化网络键值存储系统 —— 服务器端")]
struct Args {
    /// 监听地址
    #[arg(short, long, default_value = "127.0.0.1:7878")]
    addr: String,

    /// 数据文件路径
    #[arg(short, long, default_value = "data/kv.json")]
    data_file: String,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = server::run(server::Config::new(args.addr, args.data_file)) {
        eprintln!("服务器启动失败: {}", e);
        std::process::exit(1);
    }
}
