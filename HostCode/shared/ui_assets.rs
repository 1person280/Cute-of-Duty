//! HUD 贴图资源：一次性把 `assets/ui/` 旧贴图交给 AssetServer 加载，供 HUD 作为底图。
//!
//! 设计动机（贴图+过程化混合）：血条/小地图边框等有现成美术底图，做好看就用；但贴图
//! 运行时路径有不确定性（本机部署工作目录不同）。因此所有 HUD 数值都**不依赖贴图**——
//! 贴图未就绪时对应 `ImageNode` 不显示，过程化彩色条/文本照样出画，绝不让玩家卡死或崩掉。

use bevy::asset::AssetServer;
use bevy::prelude::*;

use crate::flow::flow_state::CjkFont;

/// HUD 贴图句柄集合（底图 + 头像 + 小地图框等）。
///
/// `ready` 表示所有句柄对应 `Image` 资源均已加载完成，HUD 系统据此决定是否可挂钩贴图。
#[derive(Resource)]
pub struct UiAssets {
    /// 血条底图（横向拉伸为血条背景）。
    pub health_bar: Option<Handle<Image>>,
    /// 干员头像。
    pub avatar: Option<Handle<Image>>,
    /// 弹药/装备图标。
    pub gear: Option<Handle<Image>>,
    /// 小地图外框。
    pub minimap_frame: Option<Handle<Image>>,
    /// 撤离引导小图标。
    pub extraction_icon: Option<Handle<Image>>,
    /// 是否全部就绪。
    pub ready: bool,
}

impl UiAssets {
    /// 以 `AssetServer` 请求加载全部贴图（句柄立即返回，资源后台异步就绪）。
    fn begin_load(asset_server: &AssetServer) -> Self {
        let load = |path: &'static str| Some(asset_server.load(path));
        Self {
            health_bar: load("ui/health_bar.png"),
            avatar: load("ui/yanhu_avatar.png"),
            gear: load("ui/gear_icon.png"),
            minimap_frame: load("ui/minimap_frame.png"),
            extraction_icon: load("ui/extraction_icon.png"),
            ready: false,
        }
    }

    /// 更新就绪标记：仅当所有句柄对应的 `Image` 均已加载（`CjkFont` 就绪才能画字）。
    fn refresh_ready(&mut self, images: &Assets<Image>, fonts: &CjkFont) {
        if fonts.0.is_none() {
            self.ready = false;
            return;
        }
        let handles = [
            self.health_bar.as_ref(),
            self.avatar.as_ref(),
            self.gear.as_ref(),
            self.minimap_frame.as_ref(),
            self.extraction_icon.as_ref(),
        ];
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