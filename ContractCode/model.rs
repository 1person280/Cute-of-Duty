//! 模型身份与体素几何契约（随快照/资源通道下发的稳定结构）
//!
//! 设计动机（Why）：[`ModelPreset`] 随每帧快照的 `model_preset` 下发；[`VoxelModelSpec`] /
//! [`VoxelAnimationSpec`] 经资源通道下发到客户端内存。三者都是**跨端契约**。
//! 而"从哪份源文件解析出几何"（`ServerCode::model::loader` 的 `catalog()` / `animations()`）
//! 属服务端易变内容，留在服务端。

use serde::{Deserialize, Serialize};

/// 设计像素 → 世界米（32px 身高 ≈ 3.52m，与通用方块人身高相当）。
pub const YANHU_SCALE: f32 = 0.11;

/// 体素模型身份（服务端权威标记，经快照下行给客户端）
///
/// 命名即"这个实体的胸口形象"，客户端依此从本地资产挑选体素网格与配色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelPreset {
    /// 我方火系干员（默认玩家形象）
    OperativeFire,
    /// 我方冰系干员
    OperativeIce,
    /// 我方电系干员
    OperativeElectric,
    /// 我方毒系干员
    OperativePoison,
    /// 敌方通用暴徒
    EnemyThug,
    /// 手雷投射物
    Grenade,
    /// 补给箱（搜刮点）
    SupplyCrate,
    /// 场景障碍/掩体
    Obstacle,
    /// 训练靶（实弹靶机造型）
    AimTarget,
    /// 功能站点（补给台/干员切换台/物资箱的矮台造型）
    Station,
}

/// 单个盒子（设计像素空间：`origin` 为角点、`size` 为三边长度）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelCube {
    /// 角点坐标（像素，脸朝 -Z）。
    pub origin: [f32; 3],
    /// 三边尺寸（像素）。
    pub size: [f32; 3],
    /// 逐盒材质键（语义名，如 `hair` / `cream` / `eye`）：客户端据此取冷资源色。
    ///
    /// 设计动机（Why）：只按**骨名**着色会让同一根骨上的所有盒子同色——发冠/刘海/腰带/
    /// 靴子/护肩等细节全部被染成躯干或皮肤色，模型退化成一坨纯色方块（0.3.2 参考实现是
    /// **逐盒材质**，故造型才有层次）。材质键随几何一并下发，造型决定权仍完全在服务端。
    /// `None` 表示未标注，客户端退回按骨名的兜底着色。
    #[serde(default)]
    pub mat: Option<String>,
}

/// 单根骨骼（枢轴 + 静置旋转 + 挂载的盒子）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelBone {
    /// 骨骼名（如 `head` / `tail_a`；客户端动作系统按名寻址）。
    pub name: String,
    /// 父骨名（`None` = 直接挂根）。
    pub parent: Option<String>,
    /// 枢轴点（像素）：骨骼旋转绕此点进行。
    pub pivot: [f32; 3],
    /// 静置旋转（度，XYZ 欧拉）。
    pub rotation: [f32; 3],
    /// 该骨挂载的盒子。
    pub cubes: Vec<VoxelCube>,
}

/// 一套体素模型规格（服务端权威，经协议下行给客户端渲染）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelModelSpec {
    /// 该规格服务的模型身份。
    pub preset: ModelPreset,
    /// 设计像素 → 世界米比例。
    pub scale: f32,
    /// 骨骼列表（父子关系写在 `parent` 里，客户端据此搭实体树）。
    pub bones: Vec<VoxelBone>,
}

/// 单个旋转轴上的取值：既可能是常量，也可能是 `math.sin(...)` 表达式（原样保留 YSM 语法）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RotationExpr {
    /// 常量角度（度）。
    Const(f32),
    /// YSM 表达式字符串（含 `query.anim_time` 等，由客户端求值）。
    Expr(String),
}

/// 单骨动画轨道（三轴旋转表达式）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelBoneTrack {
    /// 骨骼名。
    pub bone: String,
    /// XYZ 三轴旋转表达式。
    pub rotation: [RotationExpr; 3],
}

/// 一段动画 clip（循环标记 + 时长 + 各骨轨道）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelAnimationSpec {
    /// clip 名（如 `animation.firefox.idle`）。
    pub clip: String,
    /// 动画时长（秒）。
    pub length: f32,
    /// 该 clip 涉及的骨骼轨道。
    pub tracks: Vec<VoxelBoneTrack>,
}