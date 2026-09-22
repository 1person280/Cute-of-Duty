//! 射击区（打，z -200..200）—— 玩家"搜"完后进入本区进行远程交战。
//!
//! 每 100m 一条警戒色距离标线（200/100/0/-100/-200），三排射击道
//! （x=-200/0/200）逐级抬高的静态靶 + 两具横移/巡逻移动目标。标线为
//! decor（无碰撞），射击道全净空，保证 400m 级直射视线不被切断。

use crate::map::{MaterialKind, Motion, Prop, TargetSpec};

/// 距离标线（虚线）+ 两侧标牌柱（立柱 + 白板标距）
pub(crate) fn zone_markers() -> Vec<Prop> {
    let mut v = Vec::new();
    let lines = [
        (200.0, MaterialKind::WarningYellow),
        (100.0, MaterialKind::WarningOrange),
        (0.0, MaterialKind::WarningRed),
        (-100.0, MaterialKind::WarningOrange),
        (-200.0, MaterialKind::WarningYellow),
    ];
    for (z, mat) in lines {
        for x in -12..=12 {
            v.push(Prop::decor([x as f32 * 40.0, 0.02, z], [16.0, 0.02, 0.12], mat));
        }
        // 两侧标牌柱（不影响中部射击道净空）
        for sx in [-460.0, 460.0] {
            v.push(Prop::decor([sx, 1.5, z], [0.12, 1.5, 0.12], MaterialKind::Steel));
            v.push(Prop::decor([sx, 2.4, z + 0.2], [0.6, 0.35, 0.06], MaterialKind::PaintWhite));
        }
    }
    v
}

/// 三排射击道静态靶：近（100m）/ 中（200m）/ 远（300m），逐级抬高
pub(crate) fn engage_targets() -> Vec<TargetSpec> {
    vec![
        TargetSpec { pos: [-200.0, 1.5, 100.0], label: "射击目标", motion: None },
        TargetSpec { pos: [0.0, 1.5, 100.0], label: "射击目标", motion: None },
        TargetSpec { pos: [200.0, 1.5, 100.0], label: "射击目标", motion: None },
        TargetSpec { pos: [-200.0, 1.8, 0.0], label: "射击目标", motion: None },
        TargetSpec { pos: [0.0, 1.8, 0.0], label: "射击目标", motion: None },
        TargetSpec { pos: [200.0, 1.8, 0.0], label: "射击目标", motion: None },
        TargetSpec { pos: [-200.0, 2.2, -100.0], label: "射击目标", motion: None },
        TargetSpec { pos: [0.0, 2.2, -100.0], label: "射击目标", motion: None },
        TargetSpec { pos: [200.0, 2.2, -100.0], label: "射击目标", motion: None },
    ]
}

/// 移动目标：150m 处快速横移 + 250m 处中速巡逻（扫掠不越界不穿掩体）
pub(crate) fn engage_movers() -> Vec<TargetSpec> {
    vec![
        TargetSpec {
            pos: [0.0, 1.8, 50.0],
            label: "移动目标",
            motion: Some(Motion { speed: 4.0, range: 40.0, start_dir: -1.0 }),
        },
        TargetSpec {
            pos: [0.0, 1.8, -50.0],
            label: "移动目标",
            motion: Some(Motion { speed: 3.0, range: 60.0, start_dir: 1.0 }),
        },
    ]
}
