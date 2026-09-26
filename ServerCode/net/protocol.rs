//! 服务器权威线格式协议（TCP + NDJSON 帧）
//!
//! 这是客户端(表现层)与服务器(权威)之间的唯一契约。客户端不依赖任何服务端模拟逻辑，
//! 仅靠本模块定义的数据结构反序列化服务端算好的结果来显示——这正是
//! README「4.2 服务器权威架构」所要求的：所有热数据由服务端计算，客户端只展示。

use serde::{Deserialize, Serialize};

use crate::element::EntityElementState;
use crate::equipment::{EquipmentElement, EquipmentTier, EquipmentType};
use crate::model::ModelPreset;

/// 权威快照中的单个实体条目。
///
/// 值全部自包含（坐标/血量/element_state/模型身份），客户端只需反序列化即可渲染，
/// 无需感知服务端实体内部结构。`model_preset` 亦为服务端权威：客户端不挑选造型，
/// 只用本地稳定几何按此身份绘制。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    /// 服务端权威实体 ID（客户端据此区分物体）
    pub entity_id: u64,
    /// 世界坐标（服务器权威，客户端不得本地校订位置）
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// 当前血量
    pub hp: f32,
    /// 存活标记
    pub is_alive: bool,
    /// 模型身份（由服务端裁决，客户端只做冷映射渲染）
    pub model_preset: ModelPreset,
    /// 元素附着状态（直接复用核心库枚举，serde 已 derive，单一事实来源）
    pub element_state: EntityElementState,
    /// 当前护甲值（HUD 血条上方护甲条；服务端权威）
    pub armor: f32,
    /// 当前手持武器槽位索引（HUD 武器槽；服务端权威）
    pub weapon_index: u8,
    /// 当前武器剩余弹药（HUD 弹药数字；服务端权威；-1 表示非玩家实体无弹药）
    pub ammo: i32,
    /// 干员编号（HUD 干员名；服务端权威）
    pub operator_id: u32,
    /// Q 技能剩余冷却（0=可用；HUD 技能冷却盘）
    pub skill_cd_q: f32,
    /// E 技能剩余冷却（0=可用；HUD 技能冷却盘）
    pub skill_cd_e: f32,
}

/// 客户端→服务端的玩家意图输入。
///
/// 设计动机：长 TTK + 服务器权威，客户端只上报意图（按键/朝向），
/// 位置与伤害由服务端结算后经快照回传，天然杜绝穿墙/刷物品等外挂。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    /// 单调递增序号，服务端可用作有序回放与去重
    pub seq: u64,
    // —— 位移意图（8 向 + 跳跃 + 疾跑）——
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
    pub sprint: bool,
    /// 越肩瞄准（按住右键）：服务端据此把移速压到 `AIM_MULT` 倍，防止"瞄准中全速冲刺"。
    pub aim: bool,
    // —— 朝向意图（服务端据此旋转权威实体并结算视线）——
    /// 俯仰角（弧度，+ 上 - 下）
    pub aim_pitch: f32,
    /// 偏航角（弧度）
    pub aim_yaw: f32,
    // —— 触发/交互意图 ——
    pub shoot: bool,
    pub reload: bool,
    pub interact: bool,
    pub inventory: bool,
    pub use_item: bool,
    /// 轮盘选中项（-1=无选择）
    pub wheel_pick: i8,
    pub skill_q: bool,
    pub skill_e: bool,
}

/// 客户端→服务端上行消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// 连接握手：携带玩家档案名
    Connect { profile: String },
    /// 单帧输入意图
    Input { player: PlayerInput },
    /// 背包 CRUD 意图（服务端权威结算后经 Event 回执播报）
    Inventory { action: InventoryAction },
    /// 仓库选装确认：上报本局携带清单（服务端存档于会话热副本）
    Loadout { carried: Vec<String> },
    /// 由仓库确认进入训练场
    StartTraining,
    /// 请求撤离（服务端按玩家到撤离点距离做权威判定）
    ExtractRequest,
    /// 切换当前干员（服务端按名册索引裁决元素亲和/技能；快照 `operator_id` 跟随更新）
    SwitchOperator { operator_id: u32 },
    /// 延迟探测：seq 原样回显于 `ServerMessage::Pong`
    Ping { seq: u64 },
    /// 主动断开
    Disconnect,
}

/// 背包 CRUD 动作（客户端只上报意图，元素/数值/上限全部由服务端裁决）。
///
/// 设计动机（Why）：锻造装备的元素（随机/可指定）、背包上限、货币增减校验
/// 都属于"应该算什么"的服务端权威责任；客户端绝不携带结算结果，只给参数。
/// 线格式仍为 JSON（`serde_json`），与快照/事件共用一条 TCP 通道。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InventoryAction {
    /// 锻造一件装备。元素由服务端按等级规则生成：1 级无元素、2-6 级真随机可指定、
    /// 7-9 级真随机不可指定。
    Craft {
        name: String,
        eq_type: EquipmentType,
        tier: EquipmentTier,
        base_value: u32,
        /// 博弈区指定元素（成本翻倍）；非博弈区传 `None` 由服务端裁决
        specified_element: Option<EquipmentElement>,
    },
    /// 丢弃背包第 `index` 件，回收对应装备实例与格位。
    Discard { index: usize },
    /// 货币增减（软/硬通货/赛季代币，负数表示扣除；余额不足时服务端拒绝）。
    AdjustCurrency { soft: i64, hard: i64, season: i64 },
}

/// 单向事件（用于 HUD 播报：击杀/受击/拾取/区域通告）。
///
/// 设计动机：快照承载每帧持续状态（位置/血量），而击杀、拾取、爆头这类
/// "瞬时、一次性的播报"不该在快照里反复携带，否则客户端无法区分新旧。（Why）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    /// 击杀了目标（击杀播报 + 连击计数）
    Kill { killer_id: u64, victim_id: u64 },
    /// 被击中（受击反馈，扣血红圈 + 爆头判定）
    Hit { source_id: u64, target_id: u64, is_headshot: bool },
    /// 拾取物品
    Pickup { item_name: String },
    /// 区域/战局通告（撤离可用、抽水到账等文本）
    Announce { text: String },
}

/// 服务端→客户端下行消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// 握手应答：告知分配的权威实体 ID 与 Tick 率
    Handshake { assigned_id: u64, tick_rate_hz: u32 },
    /// 每固定 Tick 下发的权威快照（已按 AOI 兴趣区域过滤）
    Snapshot { seq: u64, entries: Vec<EntitySnapshot> },
    /// 瞬时事件（击杀/受击/拾取/通告），与快照独立、各自按序下发
    Event { kind: EventKind },
    /// 延迟探测回显（客户端据此计算到服务器往返延迟）
    Pong { seq: u64 },
    /// 撤离成功：客户端据此从训练场回主界面
    ReturnToMenu,
}

/// 便于在测试与 `broadcaster` 之外手工构造快照条目（字段较多，给默认值）。
impl EntitySnapshot {
    /// 空条目（实体成功，其余均为零值占位）。
    pub fn placeholder(id: u64) -> Self {
        Self {
            entity_id: id,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            hp: 100.0,
            is_alive: true,
            model_preset: crate::model::ModelPreset::OperativeFire,
            element_state: EntityElementState::Normal,
            armor: 0.0,
            weapon_index: 0,
            ammo: -1,
            operator_id: 0,
            skill_cd_q: 0.0,
            skill_cd_e: 0.0,
        }
    }
}

impl ClientMessage {
    /// 从一行 JSON 反序列化上行消息（解析失败给出带行的错误）。
    pub fn from_line(line: &str) -> Result<Self, String> {
        serde_json::from_str(line).map_err(|e| format!("解析客户端消息失败: {e}"))
    }
}

impl ServerMessage {
    /// 序列化为单行 JSON（帧间以换行分隔）。
    pub fn to_line(&self) -> String {
        let mut line = serde_json::to_string(self)
            .expect("ServerMessage 序列化不应失败");
        line.push('\n');
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 新玩法流程变体（选装/进场/撤离/延迟）的 JSON 往返，保证双端契约不脱节。
    #[test]
    fn gameplay_flow_variants_roundtrip() {
        let cases: Vec<ClientMessage> = vec![
            ClientMessage::Loadout {
                carried: vec!["医疗包".into(), "弹药".into()],
            },
            ClientMessage::StartTraining,
            ClientMessage::ExtractRequest,
            ClientMessage::SwitchOperator { operator_id: 2 },
            ClientMessage::Ping { seq: 7 },
        ];
        for msg in &cases {
            let js = serde_json::to_string(msg).unwrap();
            let back: ClientMessage = serde_json::from_str(&js).unwrap();
            assert_eq!(&back, msg, "ClientMessage 往返应一致: {js}");
        }

        let server_cases: Vec<ServerMessage> = vec![
            ServerMessage::Pong { seq: 7 },
            ServerMessage::ReturnToMenu,
        ];
        for msg in &server_cases {
            let line = msg.to_line();
            let back: ServerMessage = serde_json::from_str(line.trim_end()).unwrap();
            assert_eq!(&back, msg, "ServerMessage 往返应一致: {line:?}");
        }
    }
}