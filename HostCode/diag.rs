//! diag 模块 —— 渲染内存增长诊断（OOM 排查专用）
//!
//! 设计动机（Why）：README〈八〉挂账的「渲染内存缓慢增长」需要长期挂机观测数据。
//! 本模块只读不写：周期性打印实体总数与 Mesh/Material/Image/Font 资产表容量，
//! 若某列随时间单调上涨即为泄漏源方向。默认关闭，仅诊断构建时启用，
//! 不给正常游玩增加任何开销。

use bevy::prelude::*;

/// 打印周期（秒）：与 README〈八〉描述的历史诊断节奏一致（每 5s 一条）。
const PERIOD_SECS: f32 = 5.0;

/// 周期性打印存活统计。开关：环境变量 `COD_FX_TRACE=1`（启动时判定一次）。
pub fn report(
    entities: Query<Entity>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    time: Res<Time>,
    mut tick: Local<Option<Timer>>,
    mut total: Local<f32>,
) {
    if tick.is_none() {
        let enabled = std::env::var("COD_FX_TRACE").is_ok_and(|v| v == "1");
        if !enabled {
            return;
        }
        println!("[fx-trace] 时间s | 实体 | Mesh | StdMat | Image | Font");
        *tick = Some(Timer::from_seconds(PERIOD_SECS, TimerMode::Repeating));
    }
    let timer = tick.as_mut().unwrap();
    // delta 必须取真实帧时长（`Time::delta`）：传写死值会让计时器永不走满，一行都打不出。
    *total += time.delta_secs();
    if !timer.tick(time.delta()).just_finished() {
        return;
    }
    println!(
        "[fx-trace] {:>8.0} | {:>5} | {:>4} | {:>6} | {:>5} | {:>4}",
        *total,
        entities.iter().count(),
        meshes.len(),
        materials.len(),
        images.len(),
        fonts.len(),
    );
}
