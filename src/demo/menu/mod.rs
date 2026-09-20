//! 主菜单与加载屏：目录模块，按主题拆分为子模块并统一向父级导出。
//! 父模块 `use menu::*` 依赖这里的 `pub(crate) use` 聚合把所有菜单符号带入作用域。

mod loading_screen;
mod main_menu;
mod menu_behaviour;
mod mode_panel;
mod settings_panel;

pub(crate) use loading_screen::*;
pub(crate) use main_menu::*;
pub(crate) use menu_behaviour::*;
pub(crate) use mode_panel::*;
pub(crate) use settings_panel::*;