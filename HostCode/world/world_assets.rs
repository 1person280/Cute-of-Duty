//! 环境贴图资源：把旧版 `assets/environment/` 概念美术交给 AssetServer 加载，
//! 供 `world.rs` 给静态几何（墙/掩体/箱）挂 `base_color_texture` 做 PBR 打磨。
//!
//! 设计动机（贴图+底色混合）：贴图异步就绪前，材质仍保留 `base_color` 底色（bevy 在贴图
//! 未加载时照常显示底色），因此**不依赖** `ready` 也能出画面；`ready` 仅用于日志/未来
//! 需要"贴图全就绪再切换"的场合，绝不阻塞世界生成。

use bevy::asset::AssetServer;
use bevy::prelude::*;

/// 环境贴图句柄集合（全部可选，加载失败/缺失则对应材质仅用底色）。
#[derive(Resource)]
pub struct WorldAssets {
    /// 墙面/混凝土贴图（源素材：旧版 factory_bg 概念图）。
    pub wall: Option<Handle<Image>>,
    /// 掩体/装甲贴图（源素材：旧版 cover_obstacle）。
    pub cover: Option<Handle<Image>>,
    /// 箱/台面贴图（源素材：旧版 loot_crate）。
    pub crate_tex: Option<Handle<Image>>,
    /// 门框/通道贴图（源素材：旧版 doorway）。
    pub doorway: Option<Handle<Image>>,
    /// 是否全部就绪（仅信息性，材质始终依赖底色兜底）。
    pub ready: bool,
}

impl WorldAssets {
    /// 以 `AssetServer` 请求加载全部幕源贴图（句柄立即返回，资源后台异步就绪）。
    fn begin_load(asset_server: &AssetServer) -> Self {
        Self {
            wall: Some(asset_server.load("environment/factory_bg.png")),
            cover: Some(asset_server.load("environment/cover_obstacle.png")),
            crate_tex: Some(asset_server.load("environment/loot_crate.png")),
            doorway: Some(asset_server.load("environment/doorway.png")),
            ready: false,
        }
    }

    /// 刷新 `ready`：仅当所有句柄对应的 `Image` 都已加载。
    fn refresh_ready(&mut self, images: &Assets<Image>) {
        let handles = [
            self.wall.as_ref(),
            self.cover.as_ref(),
            self.crate_tex.as_ref(),
            self.doorway.as_ref(),
        ];
        self.ready = handles.iter().all(|h| match h {
            Some(h) => images.get(*h).is_some(),
            None => true,
        });
    }
}

/// 启动阶段建 `WorldAssets` 资源并触发贴图加载（须先于 `spawn_scene` 运行）。
pub fn init_world_assets(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(WorldAssets::begin_load(&asset_server));
}

/// 每帧刷新 `ready`（贴图异步就绪后置位；耗极低，常驻）。
pub fn refresh_world_ready(mut world: ResMut<WorldAssets>, images: Res<Assets<Image>>) {
    world.refresh_ready(&images);
}