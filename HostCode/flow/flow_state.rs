//! 游玩流程状态机 + 下行控制消息路由
//!
//! 设计动机（Why）：「加载 → 主菜单 → 训练场」是纯客户端表现层决策，但选装/进场/
//! 撤离的资格与距离判定一律由服务端权威裁决。本模块只负责：持有全局资源（本人实体、
//! 延迟书签、通告、中文字体、上行序号），消费 `ControlBuffer` 里的下行控制消息，
//! 并把状态机推向正确的一环。

use bevy::prelude::*;
use cute_of_duty_server::model::{VoxelAnimationSpec, VoxelModelSpec};
use cute_of_duty_server::net::protocol::{EventKind, ServerMessage};

use crate::net::network::{ClientInbound, ControlBuffer};

/// 撤离点世界坐标（草坪训练场北端撤离光垫中心；与服务端 `main::EXTRACTION_POINT`
/// 保持一致，均为地图内坐标，保证玩家可物理到达）。
pub const EXTRACTION_POINT: (f32, f32) = (0.0, -440.0);
/// 判定"进入撤离区"的触发半径（平面距离，忽略 Y）。
pub const EXTRACTION_RANGE: f32 = 12.0;
/// 加载兜底时长：握手未达也不让玩家卡死在加载屏（如服务器未启动）。
const LOADING_TIMEOUT_SECS: f32 = 3.0;

/// 客户端表面的三态游玩流程。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
pub enum AppState {
    /// 连接握手（显示加载提示）。
    #[default]
    Loading,
    /// 主菜单（含仓库选装浮层）。
    MainMenu,
    /// 训练场（HUD + 撤离判定）。
    InGame,
}

/// 中文字体资源（`zcool_kuaile.ttf` 以 include_bytes 内嵌，注册进 `Assets<Font>`）。
#[derive(Resource, Default)]
pub struct CjkFont(pub Option<Handle<Font>>);

/// 本人权威实体（握手后由路由系统填充；HUD/相机/撤离距离据此索引快照）。
#[derive(Resource, Default)]
pub struct LocalPlayer {
    pub entity_id: u64,
}

/// 服务端下发的体素模型目录（握手后一次性灌入的内存副本）。
///
/// 设计动机（Why）：几何/动画属服务端易变内容，客户端不持有源文件；本资源保存
/// `ServerMessage::ModelCatalog` 下发的规格，供 `world::voxel_model` 渲染本人模型、
/// `world::voxel_idle` 求值动画。"谁用哪个模型"仍由快照 `model_preset` 裁决。
#[derive(Resource, Default)]
pub struct ModelCatalog {
    /// 体素模型几何（骨/盒/枢轴）。
    pub models: Vec<VoxelModelSpec>,
    /// 体素动画 clip（骨名 → 旋转表达式）。
    pub animations: Vec<VoxelAnimationSpec>,
}

/// 本端视角姿态（鼠标自由视角的偏航/俯仰）。
///
/// 设计动机（Why）：这是**表现层 + 上行瞄准意图**的唯一载体——相机据此摆放，
/// 上行输入据此把 `aim_yaw/aim_pitch` 报给服务端（弹道与移动轴系的权威输入）。
/// 约定与服务端 `shooter::try_fire` 完全一致：方向 = `(sinY·cosP, sinP, cosY·cosP)`，
/// 即 **pitch 为正 = 抬头**、yaw 为正按右手系绕 Y。默认 yaw=π 面向 -Z（北/撤离区），
/// 与服务端出生朝向一致，保证进场后 W 即朝撤离点前进。
#[derive(Resource)]
pub struct AimRig {
    /// 水平偏航（弧度）。
    pub yaw: f32,
    /// 俯仰（弧度，+ 抬头 / - 低头），由系统钳制在合理区间。
    pub pitch: f32,
    /// 是否按住越肩瞄准（右键）：相机据此收臂收 FOV，上行输入据此报 `aim` 意图。
    pub aiming: bool,
}

impl Default for AimRig {
    fn default() -> Self {
        Self {
            yaw: std::f32::consts::PI,
            pitch: 0.0,
            aiming: false,
        }
    }
}

/// 连接起点 → 握手完成耗时（含 TCP 三次握手 + 首轮往返），延迟面板首行。
#[derive(Resource, Default)]
pub struct ConnectLatency {
    pub ms: f32,
}

/// 常态 Ping/Pong 往返延迟（毫秒；由网络线程测量后经 `ClientInbound::Rtt` 灌入）。
#[derive(Resource, Default)]
pub struct LiveLatency {
    pub rtt_ms: f32,
}

/// 服务端 `Event::Announce` 通告队列（HUD/菜单滚动展示）。
#[derive(Resource, Default)]
pub struct Announcements(pub Vec<String>);

/// 本人累计击杀数（仅由服务端 `EventKind::Kill` 中 `killer_id == 本人` 递增；
/// 计数权威在服务端，客户端只做展示聚合）。
#[derive(Resource, Default)]
pub struct KillCount(pub u32);

/// 上行序号（Input/Ping 公用单调递增，服务端可据此去重回放）。
#[derive(Resource, Default)]
pub struct SeqCounter(pub u64);

/// 初始化全局资源与中文字体（Startup；UI 系统在句柄就绪前会自担跳过本帧）。
pub fn setup_global(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    if let Ok(font) = Font::try_from_bytes(include_bytes!("../menu/zcool_kuaile.ttf").to_vec()) {
        commands.insert_resource(CjkFont(Some(fonts.add(font))));
    }
    commands.insert_resource(LocalPlayer::default());
    commands.insert_resource(ConnectLatency::default());
    commands.insert_resource(LiveLatency::default());
    commands.insert_resource(Announcements::default());
    commands.insert_resource(KillCount(0));
    commands.insert_resource(SeqCounter(0));
    commands.insert_resource(AimRig::default());
    commands.insert_resource(crate::net::latency::LatencyShow(false));
    commands.insert_resource(crate::net::latency::PanelSpawned(false));
}

/// 本地文本样式描述（bevy 0.15 适配层）。
///
/// Why：0.15 删除了引擎的 `TextBundle` / `TextStyle`，文本实体改为 `Text` +
/// `TextFont` + `TextColor` 三组件组合。为让既有调用点从
/// `TextBundle::from_section(txt, style(..))` 平移到新 API 而又不改写参数结构，
/// 本结构承载与旧 `TextStyle` 等价的「字体句柄 / 字号 / 颜色」三要素；仅由本模块
/// 的 `style()` 构造、`text()` 消费（字段私有）。
pub struct TextStyle {
    font: Handle<Font>,
    font_size: f32,
    color: Color,
}

/// 构建本地文本样式（字体未就绪则退默认句柄，bevy 告警并回退默认字体）。
pub fn style(fonts: &CjkFont, size: f32, color: Color) -> TextStyle {
    TextStyle {
        font: fonts.0.clone().unwrap_or_default(),
        font_size: size,
        color,
    }
}

/// 生成 0.15 文本实体组合（`Text` + `TextFont` + `TextColor`）。
///
/// Why：`Text` 自带 `#[require(Node, TextLayout, TextFont, TextColor, ..)]`，返回元组
/// 即可直接 `spawn` / `with_children`，替代已不可构造的 `TextBundle::from_section`；
/// 调用点因此只需把 `TextBundle::from_section` 改名为 `flow::text`，参数保持不变。
pub fn text(contents: impl Into<String>, style: TextStyle) -> impl Bundle {
    (
        Text::new(contents),
        TextFont {
            font: style.font,
            font_size: style.font_size,
            ..default()
        },
        TextColor(style.color),
    )
}

/// 路由下行控制消息：握手 → 记握延迟并进入主菜单；Rtt → 刷新面板延迟；
/// Announce → 入队通告；撤离成功 → 回主菜单。纯客户端编排，不触碰权威判定。
///
/// 同时承担 Loading 兜底：`LOADING_TIMEOUT_SECS` 内未握手也放行进主菜单，
/// 让服务器未启动时玩家仍能看到界面（而非永久卡加载）。
#[allow(clippy::too_many_arguments)]
pub fn route_control_messages(
    controls: ResMut<ControlBuffer>,
    mut next_state: ResMut<NextState<AppState>>,
    state: Res<State<AppState>>,
    mut player: ResMut<LocalPlayer>,
    mut connect: ResMut<ConnectLatency>,
    mut live: ResMut<LiveLatency>,
    mut announces: ResMut<Announcements>,
    mut kills: ResMut<KillCount>,
    mut load_timer: Local<f32>,
    time: Res<Time>,
) {
    // 1) 排空下行控制通道（锁只覆盖通道读取，短临界区）。
    {
        let rx = controls.0.lock().unwrap();
        while let Ok(inbound) = rx.try_recv() {
            match inbound {
                ClientInbound::Connected { assigned_id, connect_ms } => {
                    player.entity_id = assigned_id;
                    connect.ms = connect_ms;
                    // 不在 Loading 态强切主菜单：留给 `loading_tick` 持有加载屏至最短可见时长
                    // 再切换（否则握手极快时加载屏一闪而过），此判定保持服务端握手驱动。
                }
                ClientInbound::Rtt(rtt_ms) => {
                    live.rtt_ms = rtt_ms;
                }
                ClientInbound::Server(ServerMessage::Event {
                    kind: EventKind::Announce { text },
                }) => {
                    announces.0.push(text);
                }
                ClientInbound::Server(ServerMessage::Event {
                    kind: EventKind::Kill { killer_id, .. },
                }) => {
                    // 只累计本人击杀（Kill 事件权威由服务端广播；victim 忽略）。
                    if killer_id == player.entity_id {
                        kills.0 += 1;
                    }
                }
                ClientInbound::Server(ServerMessage::ReturnToMenu) => {
                    next_state.set(AppState::MainMenu);
                }
                // ModelCatalog 已改为经**远程对象池**（Resource 帧）抵达，由
                // `net::sync_catalog_from_pool` 灌入 `ModelCatalog` 视图，不再走控制通道。
                _ => {}
            }
        }
    }

    // 2) 加载兜底：超时未握手也放行进主菜单。
    if *state.get() == AppState::Loading {
        *load_timer += time.delta_secs();
        if *load_timer > LOADING_TIMEOUT_SECS {
            *load_timer = 0.0;
            next_state.set(AppState::MainMenu);
        }
    }
    let _ = &mut announces;
}