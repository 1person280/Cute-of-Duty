//! 激光瞄准线与曳光弹（**仅本人本地可见**的表现层）
//!
//! 设计动机（Why）：固定长线穿墙一眼假，必须按掩体截断；命中点取「地形/掩体交点」与
//! 「目标实体球面交点」的**近者**。体素/掩体求交用射线步进 marching（与手雷预览同思路），
//! 每帧一次成本可忽略。本模块纯为表现：弹道起点/方向/命中几何与服务端 `shooter::try_fire`
//! 同源对齐，但不参与任何判定——真实命中仍由服务端结算。
//!
//! 协议留待点：跨玩家曳光需协议新增 `Shot` 事件广播（开火点/命中点），当前未实现，
//! 留待下版本（见 ServerCode `combat/shooter.rs` 模块头）。

use bevy::prelude::*;
use cute_of_duty_contract::model::ModelPreset;

use crate::flow::{AimRig, LocalPlayer, WorldCatalog};
use crate::hud::HeldGrenadeState;
use crate::net::snapshot::SnapshotBuffer;

/// 瞄准线最大长度（米；服务端 `RAY_MAX_RANGE` 同量级，表现层自持常量）。
const AIM_RANGE: f32 = 60.0;
/// 掩体步进步长（米）：短到不穿透薄掩体，长到 60m 只需 ~240 步。
const STEP: f32 = 0.25;
/// 眼高（米）：与服务端射手出手点一致（脚底 +1.5）。
const EYE: f32 = 1.5;
/// 躯干命中球半径（米；对齐服务端 AI 躯干球）。
const TORSO_R: f32 = 1.5;
/// 头部弱点核半径（米；对齐服务端弱点头球量级）。
const HEAD_R: f32 = 0.35;
/// 枪口前移（米）：曳光起点自出手点沿视线前移的近似枪口位置。
const MUZZLE_FWD: f32 = 0.6;
/// 曳光存续时长（秒）：开火后短暂亮起随即熄灭（实机反馈 0.12s 太短难察觉，加长）。
const TRACER_SECS: f32 = 0.3;
/// 激光线颜色（细红、低透明度由 gizmo 颜色 alpha 表达）。
const LINE_COLOR: Color = Color::srgba(1.0, 0.15, 0.15, 0.45);
/// 曳光颜色（更亮更实）。
const TRACER_COLOR: Color = Color::srgba(1.0, 0.55, 0.20, 0.85);

/// 曳光瞬时状态：开火瞬间记录命中终点并起计时，超时熄灭。
#[derive(Resource, Default)]
pub(crate) struct Tracer {
    /// 曳光终点（开火瞬间的截断点）；`None` = 当前无曳光。
    end: Option<Vec3>,
    /// 剩余显示秒数。
    remaining: f32,
}

/// 激光瞄准线 + 曳光绘制（每帧）。
///
/// 持雷时不画（雷有抛物线预览，两线叠加互相干扰）；本人快照暂缺（AOI 对账间隙）时跳过。
#[allow(clippy::too_many_arguments)]
pub fn draw_aim_line(
    rig: Res<AimRig>,
    held: Res<HeldGrenadeState>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    catalog: Res<WorldCatalog>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut tracer: ResMut<Tracer>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    // 本人条目缺失时不画，避免用错误起点误导。
    let Some(me) = snap.current.iter().find(|e| e.entity_id == player.entity_id) else {
        return;
    };
    let origin = Vec3::new(me.x, me.y + EYE, me.z);
    // 方向约定与服务端 shooter::try_fire 完全一致（含俯仰）。
    let cy = rig.yaw.sin() * rig.pitch.cos();
    let dir = Vec3::new(cy, rig.pitch.sin(), rig.yaw.cos() * rig.pitch.cos());
    let end = truncate(origin, dir, &snap, player.entity_id, &catalog);

    if held.element.is_none() {
        // 激光瞄准线：常驻细红线，止于掩体或目标表面。
        gizmos.line(origin, end, LINE_COLOR);
        // 开火曳光：按住左键即近似连发——上一发熄灭后立即点亮下一发
        // （本地左键近似服务端射速节拍；单击 edge 由 remaining>0 自然覆盖）。
        if mouse.pressed(MouseButton::Left) && tracer.remaining <= 0.0 {
            tracer.end = Some(end);
            tracer.remaining = TRACER_SECS;
        }
    }

    // 曳光：枪口（出手点沿视线前移一小段）收敛到命中点的短暂亮线。
    if tracer.remaining > 0.0 {
        tracer.remaining -= time.delta_secs();
        if let Some(end) = tracer.end {
            let muzzle = origin + dir * MUZZLE_FWD - Vec3::Y * 0.1;
            gizmos.line(muzzle, end, TRACER_COLOR);
        }
    }
}

/// 视线截断：取「掩体/地形步进命中距离」与「目标实体球面命中距离」的近者。
fn truncate(
    origin: Vec3,
    dir: Vec3,
    snap: &SnapshotBuffer,
    player_entity: u64,
    catalog: &WorldCatalog,
) -> Vec3 {
    let mut best = AIM_RANGE;

    // 目标实体：快照位置求球面交点，取近者（躯干球 + 头核球；几何对齐服务端弱点评定）。
    // 真实命中判定在服务端；这里只为让视线"止于目标表面"的观感对齐。
    for e in &snap.current {
        if e.entity_id == player_entity || !e.is_alive || e.model_preset != ModelPreset::EnemyThug {
            continue;
        }
        let base = Vec3::new(e.x, e.y, e.z);
        if let Some(t) = ray_sphere(origin, dir, base, HEAD_R) {
            best = best.min(t);
        }
        if let Some(t) = ray_sphere(origin, dir, base + Vec3::Y * EYE, TORSO_R) {
            best = best.min(t);
        }
    }

    // 静态掩体/地形步进 marching。
    let mut t = STEP;
    while t < best {
        let p = origin + dir * t;
        if p.y <= 0.0 {
            best = t;
            break;
        }
        if hits_prop(p, catalog) {
            best = t;
            break;
        }
        t += STEP;
    }
    origin + dir * best
}

/// 射线与球面最近正交点（与服务端 `shooter::ray_sphere_hit` 同式）。
fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = center - origin;
    let proj = oc.dot(dir);
    let b = oc.length_squared() - radius * radius;
    let disc = proj * proj - b;
    if disc < 0.0 {
        return None;
    }
    let t0 = proj - disc.sqrt();
    if t0 > 0.0 {
        Some(t0)
    } else {
        let t1 = proj + disc.sqrt();
        (t1 > 0.0).then_some(t1)
    }
}

/// 点是否落入任一静态掩体的包围盒内（Box 按轴对齐盒；圆柱取外接盒近似，视觉可接受）。
fn hits_prop(p: Vec3, catalog: &WorldCatalog) -> bool {
    let Some(layout) = catalog.layout.as_ref() else {
        return false;
    };
    layout.props.iter().any(|prop| {
        let (hx, hy, hz) = match prop.shape {
            cute_of_duty_contract::map::Shape::Box => (prop.half[0], prop.half[1], prop.half[2]),
            cute_of_duty_contract::map::Shape::Cylinder { radius, height } => {
                (radius, height / 2.0, radius)
            }
        };
        // 旋转掩体忽略（视觉近似）；轴对齐判定足够让线"看起来止于表面"。
        (p.x - prop.pos[0]).abs() <= hx
            && (p.y - prop.pos[1]).abs() <= hy
            && (p.z - prop.pos[2]).abs() <= hz
    })
}
