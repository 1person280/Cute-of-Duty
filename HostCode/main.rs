//! Cute Of Duty 客户端（cod1）—— 渲染表现层入口
//!
//! 服务器权威架构落地：本进程只做「拉快照 + 出画」，所有热模拟数据都在服务端。
//! 表现层按子域拆成与 `launcher` 平级的六个顶层模块（`flow`/`net`/`menu`/`hud`/
//! `world`/`shared`），`launcher` 仅作装配层把它们的系统挂进 Bevy App。

mod flow;
mod hud;
mod launcher;
mod menu;
mod net;
mod shared;
mod world;

/// 服务器默认地址（与 ServerCode 默认一致）。
const DEFAULT_ADDR: &str = "127.0.0.1:8888";

fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ADDR.to_string());

    println!("Cute Of Duty 客户端（渲染表现层）→ {addr}");

    launcher::run(&addr);
}