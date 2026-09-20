//! 设置浮层标记：齿轮入口、齿轮图标与"返回"按钮组件。
//! 浮层本体在 main_menu::setup_main_menu 中随主界面一起构建。

use bevy::prelude::*;

/// 右上角齿轮按钮：打开设置浮层
#[derive(Component)]
pub(crate) struct GearButton;

/// 齿轮图标图片节点（悬停变色用）
#[derive(Component)]
pub(crate) struct GearIcon;

/// 设置浮层的"返回"按钮
#[derive(Component)]
pub(crate) struct SettingsCloseButton;