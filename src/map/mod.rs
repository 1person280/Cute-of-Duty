//! 地图模块 —— 纯数据地图定义
//!
//! 架构约定：本模块**不依赖 bevy**，只描述"地图里有什么、在哪里"，
//! 不描述"怎么画"。网格、材质、光照等渲染细节由 demo3d 的通用
//! 渲染器（`spawn_map_layout`）统一处理。
//!
//! 新增地图 = 新增 `src/map/<名称>/` 子模块，实现一个返回
//! [`MapLayout`] 的 `layout()` 函数，demo 无需改动渲染逻辑。
//!
//! 坐标约定（与 demo 一致）：x 向右 / y 向上 / z 向前，
//! 玩家出生在原点、面向 -Z；单位为米。

pub mod training;

/// 位置（米）：[x, y, z]
pub type Pos = [f32; 3];

/// 轴对齐盒体半尺寸：[hx, hy, hz]（全尺寸 = 2 × 半尺寸）
pub type HalfExtents = [f32; 3];

/// 几何形状
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// 轴对齐长方体，尺寸由 [`Prop::half`] 给出
    Box,
    /// 直立圆柱（Y 轴向），渲染尺寸自带，不使用 [`Prop::half`]
    Cylinder { radius: f32, height: f32 },
}

/// 材质语义：数据层只声明"是什么料"，颜色由渲染层映射。
/// 避免把具体色值散落在地图数据里。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialKind {
    /// 混凝土掩体
    Concrete,
    /// 锈蚀金属（集装箱/铁箱）
    Rust,
    /// 深色金属（围墙/立柱）
    Steel,
    /// 靶面红
    TargetRed,
    /// 靶面白
    TargetWhite,
    /// 白色标线/标牌
    PaintWhite,
    /// 深色细节（门框等）
    Dark,
    /// 工业管道
    Pipe,
    /// 警戒黄（近距标线）
    WarningYellow,
    /// 警戒橙（中距标线）
    WarningOrange,
    /// 警戒红（远距标线）
    WarningRed,
}

/// 发光件颜色语义（霓虹灯带/信标/出生光垫/功能台）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlowKind {
    /// 霓虹绿（后墙氛围灯）
    Green,
    /// 霓虹橙（侧墙氛围灯）
    Orange,
    /// 警示红（提取信标）
    Red,
    /// 出生光垫
    SpawnPad,
    /// 琥珀光（补给台）
    Supply,
    /// 青色光（干员切换台）
    Operator,
}

/// 静态物体：掩体、立柱、标线、装饰，一切不动的东西
#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub shape: Shape,
    pub pos: Pos,
    /// 盒体半尺寸；[`Shape::Cylinder`] 时忽略（圆柱自带尺寸）
    pub half: HalfExtents,
    pub material: MaterialKind,
    /// true = 生成碰撞体（挡子弹、阻挡移动）
    pub solid: bool,
    /// 旋转（轴 + 弧度，轴角表示）；None = 不旋转
    pub rot: Option<([f32; 3], f32)>,
}

impl Prop {
    /// 完全自定义
    pub fn new(shape: Shape, pos: Pos, half: HalfExtents, material: MaterialKind, solid: bool) -> Self {
        Self { shape, pos, half, material, solid, rot: None }
    }

    /// 实心方块（有碰撞体，可作掩体）
    pub fn solid(pos: Pos, half: HalfExtents, material: MaterialKind) -> Self {
        Self::new(Shape::Box, pos, half, material, true)
    }

    /// 实心圆柱（有碰撞体，碰撞盒取圆柱外接半尺寸）
    pub fn solid_cyl(pos: Pos, radius: f32, height: f32, material: MaterialKind) -> Self {
        Self::new(Shape::Cylinder { radius, height }, pos, [0.0; 3], material, true)
    }

    /// 纯装饰方块（无碰撞体，如标线、门框贴面）
    pub fn decor(pos: Pos, half: HalfExtents, material: MaterialKind) -> Self {
        Self::new(Shape::Box, pos, half, material, false)
    }

    /// 设置旋转（轴 + 弧度），链式调用
    pub fn with_rot(mut self, axis: [f32; 3], angle_rad: f32) -> Self {
        self.rot = Some((axis, angle_rad));
        self
    }

    /// 该物体用于碰撞/遮挡检测的半尺寸（圆柱取外接盒）
    pub fn aabb_half(&self) -> HalfExtents {
        match self.shape {
            Shape::Box => self.half,
            Shape::Cylinder { radius, height } => [radius, height / 2.0, radius],
        }
    }
}

/// 往返运动参数：沿 X 轴在 `pos.x ± range` 之间来回
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub speed: f32,
    pub range: f32,
    /// 初始方向：1.0 或 -1.0
    pub start_dir: f32,
}

/// 训练靶：静态靶或按 [`Motion`] 往返移动的靶
#[derive(Clone, Copy, Debug)]
pub struct TargetSpec {
    pub pos: Pos,
    /// 击杀播报中显示的名称
    pub label: &'static str,
    /// None = 静态靶
    pub motion: Option<Motion>,
}

/// 拾取物类型（数据层枚举，渲染层映射为具体道具与颜色）
#[derive(Clone, Copy, Debug)]
pub enum PickupKind {
    Ammo { amount: i32 },
    Health { amount: f32 },
    Armor { amount: f32 },
    /// 元素手雷，直接复用核心库的元素类型
    Grenade { element: crate::element::ElementType },
    /// 步枪武器（拾取后进入背包武器架，在背包中装备）
    Weapon { element: crate::element::ElementType },
}

/// 功能站点类型：玩家靠近按 F 打开交互面板
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationKind {
    /// 无限物资补给台
    SupplyTable,
    /// 干员切换台
    OperatorDesk,
}

/// 场景功能站点（交互台本体几何由 props 提供，这里只登记位置与语义）
#[derive(Clone, Copy, Debug)]
pub struct StationSpec {
    pub pos: Pos,
    pub kind: StationKind,
    pub label: &'static str,
}

/// 场上拾取物
#[derive(Clone, Copy, Debug)]
pub struct PickupSpec {
    pub pos: Pos,
    pub label: &'static str,
    pub kind: PickupKind,
}

/// 发光件：霓虹灯带、信标、出生光垫
#[derive(Clone, Copy, Debug)]
pub struct GlowSpec {
    pub shape: Shape,
    pub pos: Pos,
    pub half: HalfExtents,
    pub glow: GlowKind,
}

/// 一张地图的完整描述：渲染层据此生成全部场景实体
#[derive(Clone, Debug)]
pub struct MapLayout {
    pub name: &'static str,
    /// 正方形场地半径（米）：地面与围墙覆盖 [-half, +half]
    pub half_extent: f32,
    /// 棋盘格地面边长
    pub floor_tile: f32,
    pub player_spawn: Pos,
    pub props: Vec<Prop>,
    pub targets: Vec<TargetSpec>,
    pub pickups: Vec<PickupSpec>,
    pub glows: Vec<GlowSpec>,
    /// 功能站点（补给台/干员切换台等交互点）
    pub stations: Vec<StationSpec>,
}
