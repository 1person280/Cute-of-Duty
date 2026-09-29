//! `WebOpsPort` 的权威实现（组合适配器）。
//!
//! 设计动机（Why）：web 模块对领域零耦合，所有"领域真相"由本适配器注入。它只做两件事：
//! ① 从 [`NetRuntime`] 读连线层真相（在线数、广播、踢人）；② 读一份由主循环每 Tick 更新的
//! **玩家展示槽**（姓名/等级来自主循环持有的热档案，web 无从也不该直接触碰）。
//!
//! 职责边界：本文件属 `net/web`（基础设施），故只依赖 `NetRuntime` 与线格式协议类型，
//! 不 `use` 任何领域模块（combat/items/…），符合铁律 4。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::net::protocol::{EventKind, ServerMessage};
use crate::net::runtime::NetRuntime;

use super::{WebError, WebOpsPort, WebPlayerInfo, WebStatus};

/// 玩家展示槽：主循环每 Tick 覆写，web 侧只读快照。
pub type PlayerSlot = Arc<Mutex<Vec<WebPlayerInfo>>>;

/// 权威运维适配器：持网络运行时 + 玩家展示槽 + 门户元信息。
pub struct AuthorityOps {
    rt: Arc<NetRuntime>,
    start: Instant,
    version: String,
    map: String,
    motd: String,
    players: PlayerSlot,
}

impl AuthorityOps {
    /// 构造适配器，返回自身与供主循环更新的玩家展示槽句柄。
    ///
    /// 在线数与运行时长在此实时计算（无需主循环喂）；仅玩家列表需主循环填。
    pub fn new(
        rt: Arc<NetRuntime>,
        version: impl Into<String>,
        map: impl Into<String>,
        motd: impl Into<String>,
    ) -> (Arc<Self>, PlayerSlot) {
        let players: PlayerSlot = Arc::new(Mutex::new(Vec::new()));
        let ops = Arc::new(Self {
            rt,
            start: Instant::now(),
            version: version.into(),
            map: map.into(),
            motd: motd.into(),
            players: Arc::clone(&players),
        });
        (ops, players)
    }
}

impl WebOpsPort for AuthorityOps {
    fn status(&self) -> WebStatus {
        WebStatus {
            version: self.version.clone(),
            online: self.rt.online_count(),
            map: self.map.clone(),
            uptime_secs: self.start.elapsed().as_secs(),
            motd: self.motd.clone(),
        }
    }

    fn players(&self) -> Vec<WebPlayerInfo> {
        self.players.lock().map(|p| p.clone()).unwrap_or_default()
    }

    fn announce(&self, text: &str) -> Result<(), WebError> {
        self.rt.send_to_all(ServerMessage::Event {
            kind: EventKind::Announce { text: text.to_string() },
        });
        Ok(())
    }

    fn kick(&self, conn_id: u64) -> Result<(), WebError> {
        // 踢人 = 关连接：投递 Disconnect（主循环回收实体/落盘档案）+ 摘除两条写队列。
        self.rt.close_transport(conn_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_reflects_online_count() {
        let rt = Arc::new(NetRuntime::new());
        let (ops, slot) = AuthorityOps::new(Arc::clone(&rt), "0.12.3", "lawn", "motd");
        assert_eq!(ops.status().online, 0);

        // 开一条非 TCP 传输（不消费下行，仅登记写队列）后在线数应 +1。
        let _handle = rt.open_transport("tester");
        assert_eq!(ops.status().online, 1);

        slot.lock().unwrap().push(WebPlayerInfo { conn_id: 100, name: "tester".into(), level: 3 });
        assert_eq!(ops.players().len(), 1);
        assert_eq!(ops.players()[0].level, 3);
    }

    #[test]
    fn kick_closes_transport() {
        let rt = Arc::new(NetRuntime::new());
        let (ops, _slot) = AuthorityOps::new(Arc::clone(&rt), "0.12.3", "lawn", "motd");
        let handle = rt.open_transport("kicky");
        assert_eq!(ops.status().online, 1);
        ops.kick(handle.conn_id).unwrap();
        assert_eq!(ops.status().online, 0);
    }
}