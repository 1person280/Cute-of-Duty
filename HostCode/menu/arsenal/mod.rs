//! 仓库选装浮层：100% 对齐 0.3.2 参考版的双清单交互「拖拽 + Shift+左键」。
//!
//! 设计动机：这是玩法决策入口。语义与旧版逐条一致——
//! - 左键按住仓库行 / 背包槽 = 开始拖拽，跟随光标出现「幽灵物资名」；
//! - 松手落在**对方容器**上才移动（拖到背包=携带，拖回仓库=取消），否则撤销；
//! - `Shift+左键` = 快捷移动（不拖拽，直接携带/取消）；
//! - 背包容许数 = `LOADOUT_CAPACITY`，仓库行内用「已携带 ✓」绿标。
//! 携带结果仅存 `ArsenalSelection` 会话热副本，确认时把清单原样上报服务端，由服务端裁决资格。
//! 客户端绝不自行结算；离开主菜单由 `StateScoped` 统一递归销毁浮层。
//!
//! 系统间冲突规避：所有 `&mut Text` / `&mut Visibility` / `&mut BackgroundColor` 查询
//! 彼此加交叉 `Without` 过滤器，保证 bevy 可在编译期证明互不相交，避免 B0001 调度 panic。
//!
//! 本文件仅作**薄网关**：按职责拆为 `state`（状态与类型）/ `layout`（装配布局）/
//! `refresh`（刷新绘制）/ `interaction`（交互拖拽）四个子模块，并原样重导出既有公开
//! 符号，保持 `crate::menu::arsenal::*` 与 `crate::menu::*` 的公开路径不变。

mod interaction;
mod layout;
mod refresh;
mod state;

pub use interaction::*;
pub use layout::*;
pub use state::*;