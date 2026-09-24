//! Cute Of Duty 客户端（cod1）—— 渲染表现层入口
//!
//! 服务器权威架构落地：本进程只做「拉快照 + 出画」，所有热模拟数据都在服务端。
//! 全部职责收敛进 `launcher` 模块（见其 mod.rs 的架构说明），本文件仅剩参数转发。

mod launcher;

/// 服务器默认地址（与 ServerCode 默认一致）。
const DEFAULT_ADDR: &str = "127.0.0.1:8888";

fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ADDR.to_string());

    println!("Cute Of Duty 客户端（渲染表现层）→ {addr}");

    launcher::run(&addr);
}