//! HUD 贴图资源：一次性把 `assets/ui/` 剩余贴图交给 AssetServer 加载，供 HUD 作为底图。
//!
//! 设计动机（贴图+过程化混合）：HUD 数值**不依赖任何贴图**——贴图未就绪时对应
//! `ImageNode` 不显示，过程化彩色条/文本照样出画，绝不让玩家卡死或崩掉。
//!
//! 精简说明（为何只剩 gear）：旧版遗留的多张 1920² 概念美术底图（血条底/头像/小地图框/
//! 撤离图标）既与体素低模美术方向相悖，又徒增显存与加载耗时，已整体废弃；仅保留体积极小的
//! `gear_icon.png`（装备页图标），其余 HUD 一律走过程化绘制。

use bevy::asset::AssetServer;
use bevy::prelude::*;

use crate::flow::flow_state::CjkFont;

/// HUD 贴图句柄集合（当前仅装备页图标）。
///
/// `ready` 表示句柄对应 `Image` 资源已加载完成，HUD 系统据此决定是否可挂钩贴图。
#[derive(Resource)]
pub struct UiAssets {
    /// 弹药/装备图标。
    pub gear: Option<Handle<Image>>,
    /// 是否就绪。
    pub ready: bool,
}

impl UiAssets {
    /// 以 `AssetServer` 请求加载全部贴图（句柄立即返回，资源后台异步就绪）。
    fn begin_load(asset_server: &AssetServer) -> Self {
        Self {
            gear: Some(asset_server.load("ui/gear_icon.png")),
            ready: false,
        }
    }

    /// 更新就绪标记：仅当所有句柄对应的 `Image` 均已加载（`CjkFont` 就绪才能画字）。
    fn refresh_ready(&mut self, images: &Assets<Image>, fonts: &CjkFont) {
        if fonts.0.is_none() {
            self.ready = false;
            return;
        }
        let handles = [self.gear.as_ref()];
        self.ready = handles.iter().all(|h| match h {
            Some(h) => images.get(*h).is_some(),
            None => true,
        });
    }
}

/// 启动阶段建 `UiAssets` 资源并触发贴图加载。
pub fn init_ui_assets(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(UiAssets::begin_load(&asset_server));
}

/// 每帧刷新 `ready`（贴图异步就绪后置位）。
pub fn refresh_ui_ready(
    images: Res<Assets<Image>>,
    mut ui: ResMut<UiAssets>,
    fonts: Res<CjkFont>,
) {
    ui.refresh_ready(&images, &fonts);
}