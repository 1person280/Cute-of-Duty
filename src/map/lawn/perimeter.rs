//! 1×1km 平地草坪训练场 —— 外圈围墙（纯数据，无 bevy 依赖）。
//!
//! 四面围墙围住 [±500, ±500] 场地；围墙是全场唯二类大尺度 solid 掩体，
//! 也是"撤"阶段唯一的边界（玩家撤到北端信标即完成循环）。

use crate::map::{MaterialKind, Prop};

use super::{HALF, WALL_H};

/// 四面边界墙：东西墙长 1000m（沿 z），南北墙长 1000m（沿 x）。
/// 半尺寸取 HALF 使墙体恰好压在场边，不外扩也不留缝。
pub(crate) fn walls() -> Vec<Prop> {
    vec![
        Prop::solid([HALF, WALL_H / 2.0, 0.0], [0.4, WALL_H / 2.0, HALF], MaterialKind::Concrete),
        Prop::solid([-HALF, WALL_H / 2.0, 0.0], [0.4, WALL_H / 2.0, HALF], MaterialKind::Concrete),
        Prop::solid([0.0, WALL_H / 2.0, HALF], [HALF, WALL_H / 2.0, 0.4], MaterialKind::Concrete),
        Prop::solid([0.0, WALL_H / 2.0, -HALF], [HALF, WALL_H / 2.0, 0.4], MaterialKind::Concrete),
    ]
}
