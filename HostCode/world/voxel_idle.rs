//! 体素 idle 动画驱动（对服务端下发动画 clip 的极小表达式求值）
//!
//! 设计动机（Why）：动画表达式是造型内容的一部分（属服务端易变内容），随 `ModelCatalog`
//! 一并下发；客户端只负责求值 + 写旋转。为避免在表现层塞进一个通用表达式引擎，本模块
//! **只覆盖几何文件实际用到的形式**：
//!   - 纯常量：`"25"` / `"-70"`；
//!   - 正弦：`[A + ] math.sin(query.anim_time * K [± C]) * M`。
//! 其中 `math.sin` 参数按度（YSM/Molang 约定），故求值前转弧度；`anim_time` 在
//! `animation_length` 内循环回绕。

use bevy::prelude::*;
use cute_of_duty_contract::model::RotationExpr;

use crate::flow::ModelCatalog;
use crate::world::voxel_model::VoxelBoneNode;

/// 本地玩家使用的动画 clip（0.10.0 仅驱动 idle；walk/hold_gun 留待后续）。
pub const IDLE_CLIP: &str = "animation.firefox.idle";

/// 驱动 idle clip：按骨名把旋转表达式求值后写入枢轴实体的 `Transform`。
///
/// 只对带 [`VoxelBoneNode`] 的实体生效（仅本人模型有该组件）；目录未到则整体跳过。
pub fn drive_idle(
    time: Res<Time>,
    catalog: Res<ModelCatalog>,
    mut query: Query<(&VoxelBoneNode, &mut Transform)>,
) {
    let Some(anim) = catalog.animations.iter().find(|a| a.clip == IDLE_CLIP) else {
        return;
    };
    if anim.length <= 0.0 {
        return;
    }
    let t = time.elapsed_secs() % anim.length;
    for (node, mut tf) in query.iter_mut() {
        let Some(track) = anim.tracks.iter().find(|tr| tr.bone == node.0) else {
            continue;
        };
        let rx = eval_axis(&track.rotation[0], t);
        let ry = eval_axis(&track.rotation[1], t);
        let rz = eval_axis(&track.rotation[2], t);
        // 与 `voxel_model::bone_quat` 同一镜射补偿：(−rx, −ry, rz)。
        tf.rotation = Quat::from_euler(
            EulerRot::XYZ,
            (-rx).to_radians(),
            (-ry).to_radians(),
            rz.to_radians(),
        );
    }
}

/// 求单个轴上的取值（常量或表达式）。
fn eval_axis(expr: &RotationExpr, t: f32) -> f32 {
    match expr {
        RotationExpr::Const(v) => *v,
        RotationExpr::Expr(s) => eval_expression(s, t),
    }
}

/// 求值 YSM 表达式（仅覆盖 `[A + ] math.sin(query.anim_time * K [± C]) * M` 与常量）。
///
/// 无法识别的输入退化为 `0.0`（表现层容错，绝不让渲染因造型数据异常而崩）。
fn eval_expression(expr: &str, t: f32) -> f32 {
    let s = expr.trim();
    let Some(sin_pos) = s.find("math.sin(") else {
        return s.parse::<f32>().unwrap_or(0.0);
    };
    // 前缀（sin 之前的常量偏移，如 "-15 + "）
    let base = leading_const(&s[..sin_pos]);
    // 括号内参数（度）
    let rest = &s[sin_pos + "math.sin(".len()..];
    let Some(close) = rest.find(')') else {
        return base;
    };
    let arg_deg = anim_arg(&rest[..close], t);
    // 后缀（sin 之后的放大倍数，如 " * 0.8"）
    let amp = trailing_mul(&rest[close + 1..]);
    base + amp * arg_deg.to_radians().sin()
}

/// 解析 sin 前的常量偏移：`"" → 0`、`"-15 + " → -15`、`"25 + " → 25`。
fn leading_const(prefix: &str) -> f32 {
    let p = prefix.trim().trim_end_matches('+').trim();
    if p.is_empty() {
        0.0
    } else {
        p.parse::<f32>().unwrap_or(0.0)
    }
}

/// 解析 sin 后的放大倍数：`"" → 1`、`" * 0.8" → 0.8`。
fn trailing_mul(suffix: &str) -> f32 {
    let s = suffix.trim();
    let s = s.strip_prefix('*').unwrap_or(s).trim();
    if s.is_empty() {
        1.0
    } else {
        s.parse::<f32>().unwrap_or(1.0)
    }
}

/// 解析 `query.anim_time * K [± C]` → 度数 `K·t + C`。
fn anim_arg(inner: &str, t: f32) -> f32 {
    let inner = inner.trim();
    let Some(after_time) = inner.split("query.anim_time").nth(1) else {
        return 0.0;
    };
    let mut k = 1.0_f32;
    let mut c = 0.0_f32;
    for (i, term) in after_time.split('+').enumerate() {
        let cleaned = term.trim().trim_start_matches('*').trim();
        if i == 0 {
            k = cleaned.parse::<f32>().unwrap_or(1.0);
        } else {
            c = cleaned.parse::<f32>().unwrap_or(0.0);
        }
    }
    k * t + c
}