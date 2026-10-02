//! 仓库选装浮层的**状态与类型**：常量清单、资源、组件标记与互斥查询别名。
//!
//! 设计动机（Why）：把「浮层拥有哪些数据」与「如何绘制 / 如何交互」分开——布局、
//! 刷新、交互三个子模块共享同一份状态定义，避免同一实体标记在各文件里各写一遍导致
//! 语义漂移。本文件只声明类型与数值，不含任何系统逻辑。

use bevy::prelude::*;

/// 仓库物资清单（对齐 0.3.2 参考版：生存/防御/补给 + 四系元素手雷）。
/// 名字原样经 `Loadout { carried }` 上报服务端；带入进图后的「生效结算」属训练场待办。
pub(super) const MVP_ITEMS: [&str; 7] = [
    "医疗包", "护甲板", "额外弹药", "烈焰手雷", "冰霜手雷", "雷电手雷", "毒素手雷",
];

/// 与 `MVP_ITEMS` 顺序一致的行内文字配色（对齐参考版按物资类型着色）。
pub(super) const ITEM_COLORS: [Color; 7] = [
    Color::srgb(0.85, 0.22, 0.22), // 医疗包·红
    Color::srgb(0.25, 0.55, 0.90), // 护甲板·蓝
    Color::srgb(0.90, 0.78, 0.30), // 额外弹药·金
    Color::srgb(0.92, 0.55, 0.20), // 烈焰手雷·橙
    Color::srgb(0.35, 0.80, 0.85), // 冰霜手雷·青
    Color::srgb(0.90, 0.78, 0.30), // 雷电手雷·黄
    Color::srgb(0.40, 0.80, 0.42), // 毒素手雷·绿
];

/// 背包容纳上限（与服务端 4×3 背包格位同口径）。
pub(super) const LOADOUT_CAPACITY: usize = 12;

/// 仓库 / 背包网格几何：4 列 × 3 行（对齐服务端 `BACKPACK_SLOTS` 的 4×3 形态）。
pub(super) const ARSENAL_COLS: usize = 4;
pub(super) const ARSENAL_ROWS: usize = 3;
pub(super) const ARSENAL_SLOTS: usize = ARSENAL_COLS * ARSENAL_ROWS;
pub(super) const CELL_W: f32 = 132.0;
pub(super) const CELL_H: f32 = 56.0;
pub(super) const CELL_GAP: f32 = 8.0;

/// 浮层左上角偏移（相对屏幕）。
pub(super) const OVERLAY_OFFSET: f32 = 120.0;

// ——— 资源 ———

/// 浮层当前是否展开（主菜单按钮 / 确认 / 返回 共同维护）。
#[derive(Resource, Default)]
pub struct ArsenalVisible(pub bool);

/// 本局已携带的物资名（确认时上报服务端；即「背包」清单，按携带顺序）。
#[derive(Resource, Default)]
pub struct ArsenalSelection(pub Vec<String>);

/// 拖拽会话状态：谁被拖起。
#[derive(Resource, Default)]
pub struct ArsenalDrag {
    pub source: Option<ArsenalDragSource>,
}

/// 拖拽源：来自仓库池（按下标）或背包槽（按下标）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArsenalDragSource {
    Warehouse(usize),
    Carried(usize),
}

// ——— 组件标记 ———
/// 浮层根节点。
#[derive(Component)]
pub struct ArsenalRoot;
/// 左「仓库」单个物资行（记录清单索引）。
#[derive(Component)]
pub struct ArsenalRow {
    pub index: usize,
}
/// 右「背包」槽（按下标 0..CAPACITY 定位携带序位）。
#[derive(Component)]
pub struct BackpackRow {
    pub index: usize,
}
/// 左列容器：作为拖拽「仓库落区」勾选目标。
#[derive(Component)]
pub struct WarehouseZone;
/// 右列容器：作为拖拽「背包落区」勾选目标。
#[derive(Component)]
pub struct BackpackZone;
/// 仓库行的「已携带 ✓」绿标文本（记录清单索引）。
#[derive(Component)]
pub struct WhStatusText(pub usize);
/// 背包槽的物资名/空 文本（记录槽位）。
#[derive(Component)]
pub struct CarriedSlotText(pub usize);
/// 背包容量计数文本。
#[derive(Component)]
pub struct BackpackCapacity;
/// 拖拽「幽灵」根节点（跟随光标）。
#[derive(Component)]
pub struct LoadoutGhost;
/// 拖拽「幽灵」物资名文本。
#[derive(Component)]
pub struct LoadoutGhostText;
/// 「开始游戏」按钮。
#[derive(Component)]
pub struct JinButton;
/// 「返回」按钮：仅收起浮层。
#[derive(Component)]
pub struct BackButton;
/// 「预设」按钮：点击以服务端下发的第 `index` 套预设覆盖携带清单。
#[derive(Component)]
pub struct PresetButton {
    pub index: usize,
}

// ——— 定制查询类型（互斥；供拖拽系统与刷新函数共用）———
/// 仓库行绿标文本（与其余 &mut Text 互斥）。
pub(super) type WhTextQ<'w, 's> = Query<
    'w,
    's,
    (&'static WhStatusText, &'static mut Text),
    (Without<CarriedSlotText>, Without<BackpackCapacity>, Without<LoadoutGhostText>),
>;
/// 背包槽文本（与其余 &mut Text 互斥）。
pub(super) type BpTextQ<'w, 's> = Query<
    'w,
    's,
    (&'static CarriedSlotText, &'static mut Text, &'static mut TextColor),
    (Without<WhStatusText>, Without<BackpackCapacity>, Without<LoadoutGhostText>),
>;
/// 容量计数文本（与其余 &mut Text 互斥）。
pub(super) type CapTextQ<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (With<BackpackCapacity>, Without<WhStatusText>, Without<CarriedSlotText>, Without<LoadoutGhostText>),
>;
/// 拖拽幽灵文本（与其余 &mut Text 互斥）。
pub(super) type GhostTextQ<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<LoadoutGhostText>, Without<WhStatusText>, Without<CarriedSlotText>, Without<BackpackCapacity>),
>;
/// 幽灵根（&mut Visibility/Style；与背包行 Visibility 互斥）。
pub(super) type GhostQ<'w, 's> = Query<
    'w,
    's,
    (&'static mut Visibility, &'static mut Node),
    (With<LoadoutGhost>, Without<ArsenalRoot>, Without<BackpackRow>),
>;