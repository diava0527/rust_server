//! 可持久化网络键值存储系统 —— 核心库
//!
//! 本库被两个二进制目标（`server` 与 `client`）共享，承载所有可复用的逻辑。
//! 采用「库 + 二进制」的经典 Cargo 组织方式：
//! 业务逻辑集中在 `src/*.rs` 的库模块中，`src/bin/` 下只放很薄的启动入口。
//!
//! 模块划分：
//!
//! | 模块           | 职责                                                   |
//! |----------------|--------------------------------------------------------|
//! | [`error`]      | 统一的错误类型，贯穿整个项目                             |
//! | [`protocol`]   | 命令模型与网络协议（请求/响应类型 + 消息读写）            |
//! | [`store`]      | 运行时键值存储（线程安全的内存态）                       |
//! | [`persistence`]| 数据持久化与重启恢复（JSON 快照）                        |
//! | [`server`]     | 服务器端处理逻辑（监听、并发、请求分发）                  |
//! | [`client`]     | 命令行客户端逻辑（解析输入、收发消息）                    |

// 骨架阶段的临时配置：各模块的函数体还是 todo!()，因此会出现
// 「参数未使用」「私有函数/结构体尚未被调用」等警告。下面两行让
// cargo build 保持零警告；实现完成后建议删除，让这些警告重新生效，
// 帮助你发现真正的死代码和未使用变量。
#![allow(dead_code, unused_variables)]

pub mod client;
pub mod error;
pub mod persistence;
pub mod protocol;
pub mod server;
pub mod store;

// 把最常用的类型在库根部再导出，方便二进制目标直接 `use kvstore::{...}`。
pub use error::{KvError, Result};
pub use protocol::{Request, Response, StatusInfo};
pub use store::Store;
