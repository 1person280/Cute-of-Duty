//! 战斗：射击、换弹、元素反应、技能与手雷、爆炸、伤害数字
//!
//! 由多个语义化子模块组成，统一在此重新导出，父模块 `use combat::*` 可直接引用全部符号；
//! 外部模块（如 `targets`、`inventory`）走 `super::combat::X` 的符号也由此接通。

mod explosion;
mod feedback;
mod grenade;
mod reaction;
mod skill;
mod weapon;
mod zone;

pub(crate) use explosion::*;
pub(crate) use feedback::*;
pub(crate) use grenade::*;
pub(crate) use reaction::*;
pub(crate) use skill::*;
pub(crate) use weapon::*;
pub(crate) use zone::*;