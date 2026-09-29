//! 模态仲裁 —— 唯一所有者 [`ModalState`]，其余模块只读、写入一律经 [`ModalChange`] 事件。
//!
//! 设计动机（Why）：客户端有七八个"打开即冻结玩法输入"的模态（暂停 / 大地图 / F 交互二级面板 /
//! 物资箱 / 径向轮盘 / 背包 / 操作按钮组 / 持雷 / Esc 软开关）。若让 `hud` 去读 `menu` 的暂停、
//! 让 `net` 去读 `hud` 的轮盘，就会出现同层横向依赖，且每新增一个模态都要在 `hud` 与 `net` 里
//! 各补一条 `&& !xxx.open`——漏一处就是"暂停时还能开枪"。
//!
//! 收敛办法：`flow` 作为应用流程状态机拥有唯一仲裁资源；生产者（`menu` / `hud`）只**发事件**，
//! 由本模块的 [`apply_modal_changes`] 落账；消费者（`hud` / `net`）只 `Res<ModalState>` 读结论。
//! 新增模态只需加一个 [`ModalKind`] 变体 + 一处事件发射，消费侧一行不用改。
//!
//! 生产者不必在各开关点手工发事件：`menu` / `hud` 各有一个"每帧对照源资源与仲裁态、有差异才发"
//! 的发布系统（见 `hud::modal::publish` 与 `menu::pause::publish_modal_changes`），
//! 把写入收敛到单一位置，避免多处分叉漂移。

use bevy::prelude::*;

/// 模态种类：每种"打开即影响输入/光标判定"的面板或状态一枚。
///
/// 设计动机（Why）：用枚举而非多个 bool 参数传递事件，使"新增模态"成为编译期可检的封闭变更，
/// 也让他模块无需知道各面板资源的字段名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    /// 暂停菜单（含设置子面板）
    Pause,
    /// 战术大地图
    BigMap,
    /// F 交互二级选项面板
    Interact,
    /// 物资箱 / 补给台 4×3 格位面板
    Loot,
    /// 3/4 消耗品径向轮盘
    Wheel,
    /// Tab 背包总览
    Backpack,
    /// B 可点击操作按钮组
    Button,
    /// 服务端权威下发的"持雷"态（影响 Esc 语义与越肩；不冻结移动）
    HeldGrenade,
    /// Esc「无 UI 时交还光标」软开关（玩家主动要求释放光标）
    CursorReleased,
}

/// 模态状态变更事实：由生产者（`menu` / `hud`）发射，[`apply_modal_changes`] 消费并落账。
#[derive(Event)]
pub struct ModalChange {
    pub kind: ModalKind,
    pub open: bool,
}

/// 「打开暂停菜单」请求：`hud`（操作按钮组）发，`menu` 消费以装配暂停 UI。
///
/// 设计动机（Why）：`hud` 不得直接 `use crate::menu::spawn_pause_ui`（同层横向）；把"请打开展开
/// 暂停"建模成事件，`menu` 作为暂停面板的唯一实现者自行装配，方向单向。
#[derive(Event)]
pub struct PauseOpenRequest;

/// 模态仲裁资源 —— **唯一所有者是 `flow`**，其它模块只读。
#[derive(Resource, Default)]
pub struct ModalState {
    pub pause: bool,
    pub bigmap: bool,
    pub interact: bool,
    pub loot: bool,
    pub wheel: bool,
    pub backpack: bool,
    pub button: bool,
    pub held_grenade: bool,
    pub cursor_released: bool,
}

impl ModalState {
    /// 读取指定模态是否打开（供生产者对照以避免重复发事件）。
    pub fn get(&self, kind: ModalKind) -> bool {
        match kind {
            ModalKind::Pause => self.pause,
            ModalKind::BigMap => self.bigmap,
            ModalKind::Interact => self.interact,
            ModalKind::Loot => self.loot,
            ModalKind::Wheel => self.wheel,
            ModalKind::Backpack => self.backpack,
            ModalKind::Button => self.button,
            ModalKind::HeldGrenade => self.held_grenade,
            ModalKind::CursorReleased => self.cursor_released,
        }
    }

    /// 落账一次模态变更（**只应由 [`apply_modal_changes`] 调用**）。
    pub fn set(&mut self, kind: ModalKind, open: bool) {
        match kind {
            ModalKind::Pause => self.pause = open,
            ModalKind::BigMap => self.bigmap = open,
            ModalKind::Interact => self.interact = open,
            ModalKind::Loot => self.loot = open,
            ModalKind::Wheel => self.wheel = open,
            ModalKind::Backpack => self.backpack = open,
            ModalKind::Button => self.button = open,
            ModalKind::HeldGrenade => self.held_grenade = open,
            ModalKind::CursorReleased => self.cursor_released = open,
        }
    }

    /// 玩法输入（移动 / 开火 / 换弹 / 技能 / 切枪）是否被冻结。
    ///
    /// 设计动机（Why）：把这个判定收敛成一问一答，消费侧（`hud` 的 `run_if`、`net` 的上报系统）
    /// 不再各自拼一串 `&& !a && !b`。持雷**不**冻结移动，故不计入。
    pub fn blocks_gameplay_input(&self) -> bool {
        self.pause
            || self.bigmap
            || self.interact
            || self.loot
            || self.wheel
            || self.backpack
            || self.button
            || self.cursor_released
    }

    /// 是否应释放鼠标光标（面板需要指针悬停/拖拽）。
    ///
    /// 设计动机（Why）：径向轮盘靠鼠标**位移**选格而非指针位置，保持锁定更符合 legacy 手感，
    /// 故把"轮盘除外"这一例外集中在本方法，避免每个调用点各写一次。
    pub fn releases_cursor(&self) -> bool {
        self.pause
            || self.bigmap
            || self.interact
            || self.loot
            || self.backpack
            || self.button
            || self.cursor_released
    }

    /// Esc 是否已被某个模态消费（其语义是"关闭该模态"），软开关不得抢用。
    ///
    /// 设计动机（Why）：轮盘由 3/4 松开收口、不消费 Esc；`cursor_released` 自身就是软开关的
    /// 结果，不应反过来阻止它。故二者排除。
    pub fn escape_consumed_by_modal(&self) -> bool {
        self.pause
            || self.bigmap
            || self.interact
            || self.loot
            || self.backpack
            || self.button
            || self.held_grenade
    }

    /// 除径向轮盘自身外，是否有其它模态打开。
    ///
    /// 设计动机（Why）：轮盘被其它模态打断时须整体丢弃本次会话（否则 `held_key` 残留会卡死），
    /// 该判定单独成法以保持 vernacular 语义清晰。
    pub fn other_modal_open(&self) -> bool {
        self.pause || self.bigmap || self.interact || self.loot || self.backpack || self.button
    }
}

/// 消费 [`ModalChange`] 事件并写入 [`ModalState`]（唯一写入点）。
pub fn apply_modal_changes(mut events: EventReader<ModalChange>, mut modal: ResMut<ModalState>) {
    for e in events.read() {
        modal.set(e.kind, e.open);
    }
}