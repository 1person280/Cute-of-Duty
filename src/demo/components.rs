//! 游戏组件与标记：实体数据定义（无逻辑），全部 crate 内共享

use bevy::prelude::*;
use crate::element::ElementType;
use crate::map::StationKind;
use crate::operator::{rifle_profile, roster};
use crate::model::{
    ready_timer, OperatorAccent, OperatorState,
};
use super::frontend::*;

#[derive(Resource, Default)]
pub(crate) struct InputState {
    pub(crate) cursor_locked: bool,
}

#[derive(Resource, Default)]
pub(crate) struct NearbyInteract {
    /// 统一交互条目：站点在最前，拾取物按距离升序排在其后
    pub(crate) entries: Vec<InteractEntry>,
    /// 当前选中的 entries 下标
    pub(crate) selected: usize,
    /// 滚动窗口起始下标：固定高度只展示 [scroll_start, scroll_start+可见行数) 的条目
    pub(crate) scroll_start: usize,
}

/// 交互菜单条目：F 执行选中项 —— 拾取物直接拾取，站点打开面板
#[derive(Clone, Copy)]
pub(crate) enum InteractEntry {
    Pickup(Entity),
    Station { kind: StationKind, label: &'static str },
}




#[derive(Component)]
pub(crate) struct Health { pub(crate) current: f32, pub(crate) max: f32 }
impl Default for Health { fn default() -> Self { Self { current: 100.0, max: 100.0 } } }

#[derive(Component)]
pub(crate) struct Armor { pub(crate) current: f32, pub(crate) max: f32 }
impl Default for Armor { fn default() -> Self { Self { current: 60.0, max: 100.0 } } }

#[derive(Component, Clone)]
pub(crate) struct WeaponData {
    pub(crate) element: ElementType,
    pub(crate) ammo: i32,
    pub(crate) max_ammo: i32,
    pub(crate) fire_interval: f32,
    pub(crate) damage: f32,
    pub(crate) name: String,
}

impl WeaponData {
    /// 从核心武器档案构建一把满弹步枪（备弹统一放背包弹药池）
    pub(crate) fn from_profile(profile: &crate::operator::RifleProfile) -> Self {
        Self {
            element: profile.element,
            ammo: profile.max_ammo,
            max_ammo: profile.max_ammo,
            fire_interval: profile.fire_interval,
            damage: profile.damage,
            name: profile.name.to_string(),
        }
    }
}

/// 主武器固定双槽：背包里永远只有两把枪，拾取新枪直接替换当前手持的那把
pub(crate) const MAX_WEAPONS: usize = 2;

/// 武器槽：只有"当前手持哪把"一个状态（weapons[0]/[1] 即 1/2 号主武器位）
#[derive(Component)]
pub(crate) struct WeaponSlot { pub(crate) current: usize }
impl Default for WeaponSlot {
    fn default() -> Self {
        Self { current: 0 }
    }
}


/// 切换干员：重置 Q/E 冷却为该干员的配置值，并把角色发光饰条染成新干员元素色
pub(crate) fn switch_operator(
    state: &mut OperatorState,
    accent: &OperatorAccent,
    materials: &mut Assets<StandardMaterial>,
    idx: usize,
) {
    let Some(op) = roster().get(idx) else { return };
    state.active = idx;
    state.q = ready_timer(op.q.cooldown_secs);
    state.e = ready_timer(op.e.cooldown_secs);
    if let Some(mat) = materials.get_mut(&accent.0) {
        mat.base_color = op.element.color();
        mat.emissive = op.element.emissive();
    }
}


#[derive(Component)]
pub(crate) struct BulletHit { pub(crate) timer: Timer }

#[derive(Component)]
pub(crate) struct DamageParticle { pub(crate) velocity: Vec3, pub(crate) timer: Timer }

#[derive(Component)]
pub(crate) struct GrenadeProjectile {
    pub(crate) velocity: Vec3,
    pub(crate) element: ElementType,
    /// 落点爆炸伤害与半径（干员技能/背包道具各自配置）
    pub(crate) damage: f32,
    pub(crate) radius: f32,
    /// 落点附加机制（点燃/冰冻/毒区，来自干员档案或背包道具默认值）
    pub(crate) effect: crate::operator::SkillEffect,
    pub(crate) timer: Timer,
}

/// 挂在目标身上的持续伤害（点燃等）：每 DOT_TICK 秒结算一次
#[derive(Clone, Debug)]
pub(crate) struct DamageOverTime {
    pub(crate) element: ElementType,
    pub(crate) dps: f32,
    pub(crate) tick: Timer,
    pub(crate) remaining: Timer,
}

/// 持续伤害的结算间隔（秒）
pub(crate) const DOT_TICK: f32 = 0.5;

#[derive(Component)]
pub(crate) struct ExplosionEffect { pub(crate) timer: Timer, pub(crate) max_scale: f32 }

#[derive(Component)]
pub(crate) struct TargetDummy {
    pub(crate) max_health: f32,
    pub(crate) current_health: f32,
    pub(crate) hit_flash: Option<Timer>,
    pub(crate) element_state: Option<ElementType>,
    pub(crate) state_timer: Option<Timer>,
    /// 倒地倒计时：Some = 已被击倒（倒地期间不可再被命中/移动）
    pub(crate) down_timer: Option<Timer>,
    /// 倒下方向（水平单位向量，取自致命一击的来弹方向）
    pub(crate) fall_dir: Vec3,
    /// 冰冻/电麻硬控：Some = 停止行动计时中（移动靶停止巡逻）
    pub(crate) frozen: Option<Timer>,
    /// 冰冻视觉（冰块）子实体，解冻/被击倒时移除
    pub(crate) frozen_visual: Option<Entity>,
    /// 持续伤害（点燃等），可叠加
    pub(crate) dots: Vec<DamageOverTime>,
    /// 击杀播报中显示的名称
    pub(crate) label: &'static str,
}
impl Default for TargetDummy {
    fn default() -> Self {
        Self {
            max_health: 200.0, current_health: 200.0,
            hit_flash: None, element_state: None, state_timer: None,
            down_timer: None, fall_dir: Vec3::X, label: "训练靶",
            frozen: None, frozen_visual: None, dots: Vec::new(),
        }
    }
}
/// 击倒后经过 fall_time 秒完全趴下，down_secs 秒后原地复活
pub(crate) const TARGET_FALL_TIME: f32 = 0.55;
pub(crate) const TARGET_DOWN_SECS: f32 = 4.0;

#[derive(Component)]
pub(crate) struct MovingTarget { pub(crate) speed: f32, pub(crate) range: f32, pub(crate) origin: Vec3, pub(crate) direction: f32 }

#[derive(Component)]
pub(crate) struct CrosshairRoot;

#[derive(Component)]
/// 准星锚点：flex 居中的 0×0 节点，准星部件相对它绝对定位（真·屏幕正中）
pub(crate) struct CrosshairAnchor;

#[derive(Component)]
pub(crate) struct CrosshairLine;

#[derive(Component)]
pub(crate) struct CrosshairCenter;

#[derive(Component)]
pub(crate) struct HudHealthBarBg;

#[derive(Component)]
pub(crate) struct HudHealthBarFill;

/// 血条下方的 "HP 100/100" 数值文本
#[derive(Component)]
pub(crate) struct HudHealthText;

#[derive(Component)]
pub(crate) struct HudArmorBarBg;

#[derive(Component)]
pub(crate) struct HudArmorBarFill;

/// 护甲条下方的 "ARMOR 60/100" 数值文本
#[derive(Component)]
pub(crate) struct HudArmorText;

#[derive(Component)]
pub(crate) struct HudAmmoMain;

#[derive(Component)]
pub(crate) struct HudAmmoReserve;

#[derive(Component)]
pub(crate) struct HudWeaponName;

#[derive(Component)]
pub(crate) struct HudWeaponSlot1;

#[derive(Component)]
pub(crate) struct HudWeaponSlot2;

#[derive(Component)]
pub(crate) struct HudSkillQBg;

#[derive(Component)]
pub(crate) struct HudSkillQFill;

#[derive(Component)]
pub(crate) struct HudSkillQText;

#[derive(Component)]
pub(crate) struct HudSkillEBg;

#[derive(Component)]
pub(crate) struct HudSkillEFill;

#[derive(Component)]
pub(crate) struct HudSkillEText;

#[derive(Component)]
pub(crate) struct HudEdgeGlow;

#[derive(Component)]
pub(crate) struct HudReloadText;

#[derive(Component)]
pub(crate) struct HudSkillQLabel;

#[derive(Component)]
pub(crate) struct HudSkillELabel;

#[derive(Component)]
pub(crate) struct FloatingReaction { pub(crate) timer: Timer }

#[derive(Component)]
pub(crate) struct DamagePopup {
    pub(crate) timer: Timer,
    pub(crate) world_pos: Vec3,
}

#[derive(Component)]
pub(crate) struct Collider { pub(crate) half_size: Vec3 }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ItemCategory {
    /// 恢复道具（3号键 / 轮盘）：医疗、护甲、弹药等补给
    #[default]
    Consumable,
    /// 战术道具（4号键 / 轮盘）：可投掷的元素手雷
    Tactical,
}

#[derive(Clone)]
pub(crate) enum PickupType {
    /// 步枪弹药：拾取时直接补充背包弹药池（不占物资槽位）
    Ammo { amount: i32 },
    Health { amount: f32 },
    Armor { amount: f32 },
    Grenade { element: ElementType },
    /// 步枪武器：拾取后进入背包武器架
    Weapon { element: ElementType },
}
impl PickupType {
    pub(crate) fn category(&self) -> ItemCategory {
        match self {
            PickupType::Grenade { .. } => ItemCategory::Tactical,
            _ => ItemCategory::Consumable,
        }
    }
}

#[derive(Clone, Component)]
pub(crate) struct PickupItem {
    pub(crate) name: String,
    pub(crate) item_type: PickupType,
}

/// 玩家背包：物资槽 + 武器架 + 弹药池。
/// 武器本体永远存放在 weapons（WeaponSlot 只存装备下标），
/// 弹药是池化资源（换弹从这里取弹，弹药拾取直接入池）。
#[derive(Component)]
pub(crate) struct Inventory {
    pub(crate) items: Vec<PickupItem>,
    pub(crate) max_slots: usize,
    pub(crate) weapons: Vec<WeaponData>,
    pub(crate) max_weapons: usize,
    pub(crate) ammo_pool: i32,
}
impl Default for Inventory {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            max_slots: 8,
            weapons: vec![
                WeaponData::from_profile(&rifle_profile(ElementType::Fire)),
                WeaponData::from_profile(&rifle_profile(ElementType::Ice)),
            ],
            max_weapons: MAX_WEAPONS,
            ammo_pool: 150,
        }
    }
}

#[derive(Component)]
pub(crate) struct InventoryUI;

/// 统一交互菜单根节点：站点 + 附近拾取物，滚轮选择 / F 确认
#[derive(Component)]
pub(crate) struct InteractMenuUI;

/// 交互菜单的面板容器（深色底）：bevy 0.14 UI 不按祖先可见性剔除，
/// 收起时需与标题/行/页脚一起显式隐藏
#[derive(Component)]
pub(crate) struct InteractMenuPanel;

/// 交互菜单第 i 个可见行槽（滚动窗口内第 i 行，选中高亮背景/边框挂在行节点上）
#[derive(Component)]
pub(crate) struct InteractRow(pub(crate) usize);

/// 交互菜单第 i 个可见行槽的文本（行节点的子实体）
#[derive(Component)]
pub(crate) struct InteractRowText(pub(crate) usize);

/// 交互菜单滚动条部件：凹槽常驻占位（保持面板宽度稳定），滑块随滚动窗口移动
#[derive(Component)]
pub(crate) struct InteractScrollBar(pub(crate) ScrollbarPart);

/// 滚动条部件角色
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ScrollbarPart {
    Track,
    Thumb,
}

/// 交互菜单标题行（F 徽标 + 标题）：bevy 0.14 UI 不按祖先可见性剔除子节点，
/// 收起菜单时必须连这一层一起显式隐藏
#[derive(Component)]
pub(crate) struct InteractMenuHeader;

/// 交互菜单底部提示行：固定操作提示 + 选中条目的满载/替换警告
#[derive(Component)]
pub(crate) struct InteractMenuHintText;

#[derive(Component)]
pub(crate) struct InventorySlotUI(pub(crate) usize);

#[derive(Component)]
pub(crate) struct InventorySlotText(pub(crate) usize);

// --- 背包扩展区（武器架 / 弹药池） ---

#[derive(Component)]
pub(crate) struct BackpackWeaponSlot(pub(crate) usize);

#[derive(Component)]
pub(crate) struct BackpackWeaponText(pub(crate) usize);

#[derive(Component)]
pub(crate) struct HudBackpackAmmo;

// --- HUD：当前干员名 ---

#[derive(Component)]
pub(crate) struct HudOperatorName;

// --- 功能站点（补给台 / 干员切换台） ---

/// 场景交互站点（几何由地图 props 提供，这里只登记语义与位置）
#[derive(Component)]
pub(crate) struct Station {
    pub(crate) kind: StationKind,
    pub(crate) label: &'static str,
}

/// 距离站点多远可交互（米）
pub(crate) const STATION_USE_RANGE: f32 = 3.0;

/// 当前打开的站点面板
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OpenStation {
    #[default]
    None,
    Supply,
    Operator,
}

#[derive(Component)]
pub(crate) struct SupplyUIRoot;

#[derive(Component)]
pub(crate) struct SupplyRow(pub(crate) usize);

#[derive(Component)]
pub(crate) struct SupplyStatusText;

#[derive(Component)]
pub(crate) struct OperatorUIRoot;

#[derive(Component)]
pub(crate) struct OperatorCard(pub(crate) usize);

#[derive(Component)]
pub(crate) struct OperatorCardText(pub(crate) usize);

#[derive(Component)]
pub(crate) struct OperatorStatusText;

/// 补给台可领取的物资行：(名称, 效果说明)
pub(crate) const SUPPLY_ROWS: [(&str, &str); 7] = [
    ("步枪弹药 ×60", "直接补充弹药池"),
    ("医疗包", "恢复 25 HP"),
    ("护甲片", "恢复 20 护甲"),
    ("烈焰手雷", "火系战术道具"),
    ("冰霜手雷", "冰系战术道具"),
    ("雷电手雷", "电系战术道具"),
    ("毒素手雷", "毒系战术道具"),
];

// --- 统一交互菜单（站点 + 拾取物） ---

/// 交互菜单固定高度内可见的行槽数（超出部分折叠，靠滚动窗口查看）
pub(crate) const INTERACT_MENU_VISIBLE_ROWS: usize = 8;
/// 交互菜单最多缓存的条目数（超过可见行数的条目靠滚动条查看）
pub(crate) const INTERACT_MENU_MAX_ENTRIES: usize = 16;
/// 交互菜单单行固定高度 / 行间距：行槽常驻占位，保证面板高度不随条目数变化
pub(crate) const INTERACT_ROW_H: f32 = 28.0;
pub(crate) const INTERACT_ROW_GAP: f32 = 4.0;
/// 滚动条凹槽高度 = 可见行总高（与条目列表列等高）
pub(crate) const INTERACT_SCROLL_TRACK_H: f32 =
    INTERACT_MENU_VISIBLE_ROWS as f32 * INTERACT_ROW_H + (INTERACT_MENU_VISIBLE_ROWS as f32 - 1.0) * INTERACT_ROW_GAP;

// --- 道具轮盘（长按 3/4 呼出） ---

#[derive(Component)]
pub(crate) struct WheelRoot;

#[derive(Component)]
pub(crate) struct WheelHubText;

/// 轮盘中心的"取消"按钮：点击撤销本次使用（不消耗道具）
#[derive(Component)]
pub(crate) struct WheelCancelButton;

#[derive(Component)]
pub(crate) struct WheelCard(pub(crate) usize);

#[derive(Component)]
pub(crate) struct WheelCardText(pub(crate) usize);

/// 轮盘状态机：按下3/4 → 短按快速使用首个对应道具，长按呼出轮盘，松开键确认使用。
/// 光标回到中心或点击中心"取消"键 → 撤销使用（不消耗道具）。
#[derive(Resource, Default)]
pub(crate) struct WheelState {
    pub(crate) open: bool,
    /// 已按下但尚未决定（短按/长按）的按键
    pub(crate) pending_key: Option<KeyCode>,
    pub(crate) pending_hold: f32,
    /// 轮盘对应的道具类别
    pub(crate) category: ItemCategory,
    /// 轮盘展示的背包条目索引（已按类别过滤）
    pub(crate) filtered: Vec<usize>,
    /// 当前选中的 filtered 下标
    pub(crate) selected: Option<usize>,
    /// 轮盘已打开时长（秒），用于超时兜底
    pub(crate) open_secs: f32,
}

/// 轮盘按键按住多久后判定为“长按”并呼出轮盘
pub(crate) const WHEEL_OPEN_DELAY: f32 = 0.28;
/// 轮盘打开多久后强制收起（兜底防卡屏）
pub(crate) const WHEEL_MAX_OPEN_SECS: f32 = 10.0;
/// 轮盘卡片环绕半径（逻辑像素）
pub(crate) const WHEEL_RADIUS: f32 = 150.0;
pub(crate) const WHEEL_CARD_W: f32 = 132.0;
pub(crate) const WHEEL_CARD_H: f32 = 44.0;

// --- 底部快捷道具图标（3 恢复 / 4 战术） ---

/// 恢复道具（3号）图标色
pub(crate) const ITEM_RECOVERY_COLOR: Color = Color::srgb(0.35, 0.85, 0.45);
/// 战术道具（4号）图标色
pub(crate) const ITEM_TACTICAL_COLOR: Color = Color::srgb(1.0, 0.62, 0.18);

#[derive(Component)]
pub(crate) struct HudItemSlotText(pub(crate) usize);

#[derive(Component)]
pub(crate) struct HudItemSlotLabel(pub(crate) usize);

// --- 击杀播报（右上角） ---

#[derive(Event)]
pub(crate) struct KillEvent {
    pub(crate) name: String,
}

#[derive(Resource, Default)]
pub(crate) struct KillStats {
    pub(crate) total: u32,
}

#[derive(Component)]
pub(crate) struct KillFeedRoot;

#[derive(Component)]
pub(crate) struct KillFeedTotal;

#[derive(Component)]
pub(crate) struct KillFeedEntry {
    pub(crate) timer: Timer,
    pub(crate) base_color: Color,
}


// =============================================================================
// World Setup

