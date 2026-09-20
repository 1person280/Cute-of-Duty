//! 配置加载模块
//!
//! 核心库统一的配置表加载入口：`src/config/` 与业务代码物理相邻，`cod1` 与 `cod1-demo` 共用。
//! 解决两类历史问题：
//! 1. 按裸相对路径读 `config/element_reactions.yaml`，换目录启动就丢配置；
//! 2. 内置默认值与 YAML 分处两地，改表后默认值漂移、行为不一致。
//!
//! 语义约定（单一事实来源）：
//! - 权威默认 = 编译期 `include_str!` 嵌入的同目录 `element_reactions.yaml`，
//!   表与代码恒同步，永不漂移，且随二进制分发，无源码也能拿到一致的默认值；
//! - 运行时覆盖 = 从项目根目录向上搜索 `src/config/element_reactions.yaml`（设计师改表热加载）；
//! - 文件缺失 → 回退嵌入默认（README 承诺的行为），返回 `None` 路径供调用方提示；
//! - 文件存在但解析失败 → 返回 `Err`，由调用方大声失败（改错表就该当场报错，而不是静默用默认值）。

use crate::element::ElementConfig;
use std::path::{Path, PathBuf};

/// 元素配置在项目内的相对路径（同时作为项目根目录的探测标记）。
///
/// 设计成"根目录/src/config/element_reactions.yaml"而非旧的全项目 `config/` 目录，
/// 是为了让配置表与加载器同处一室，物理距离最近；也作为 `project_root()` 的定位锚点。
pub const ELEMENT_CONFIG_REL: &str = "src/config/element_reactions.yaml";

/// 编译期嵌入的权威默认配置：与仓库内的 YAML 恒为同一份内容，
/// 保证"无源码环境/文件缺失"时的默认值与设计师改的表一致。
pub const ELEMENT_CONFIG_EMBEDDED: &str = include_str!("element_reactions.yaml");

/// 探测项目根目录：
/// 先从当前工作目录逐级向上找 `src/config/element_reactions.yaml`，
/// 找不到再从可执行文件所在目录逐级向上找。
/// 兼容"双击 exe""从 target/debug 启动""从任意工作目录启动"等情形。
pub fn project_root() -> Option<PathBuf> {
    if let Some(root) = std::env::current_dir().ok().and_then(|d| search_upward(&d)) {
        return Some(root);
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    search_upward(&exe_dir)
}

fn search_upward(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join(ELEMENT_CONFIG_REL).is_file() {
            return Some(d);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}

/// 定位元素配置文件（自动解析项目根目录）。
///
/// 该文件是"运行时覆盖"：存在时以其为准（可热改表），不存在时由 `load_element_config`
/// 回退至编译期嵌入的默认配置。
pub fn element_config_path() -> Option<PathBuf> {
    project_root().map(|root| root.join(ELEMENT_CONFIG_REL))
}

/// 从指定路径加载元素配置，解析失败返回带原因的 `Err`
pub fn load_element_config_from(path: &Path) -> Result<ElementConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取元素配置失败 {}: {e}", path.display()))?;
    serde_yaml::from_str(&content)
        .map_err(|e| format!("解析元素配置失败 {}: {e}", path.display()))
}

/// 便捷入口：自动定位配置文件并加载。
///
/// - 找到且解析成功 → `Ok((配置, Some(路径)))`
/// - 文件缺失       → `Ok((嵌入默认配置, None))`（编译期 `include_str!`，与 YAML 恒一致）
/// - 解析失败       → `Err(带路径与原因的完整错误信息)`
pub fn load_element_config() -> Result<(ElementConfig, Option<PathBuf>), String> {
    match element_config_path() {
        Some(path) => load_element_config_from(&path).map(|c| (c, Some(path))),
        None => serde_yaml::from_str(ELEMENT_CONFIG_EMBEDDED)
            .map(|c: ElementConfig| (c, None))
            .map_err(|e| format!("嵌入默认元素配置解析失败: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{ElementSystem, EntityElementState, ElementType};

    /// 测试环境 CWD = 包根目录，src/config/element_reactions.yaml 就在这里
    #[test]
    fn test_project_root_found() {
        let root = project_root().expect("应能定位到项目根目录");
        assert!(root.join(ELEMENT_CONFIG_REL).is_file());
    }

    #[test]
    fn test_actual_yaml_file_parses() {
        let path = element_config_path().expect("配置文件应存在");
        let config = load_element_config_from(&path).expect("随包YAML应可解析");
        // YAML 含 12 条反应（8 条元素反应 + 4 条环境修正）
        assert_eq!(config.reactions.len(), 12);
        assert_eq!(config.mutual_exclusion.self_conflicts.len(), 3);
        let teammate = config.mutual_exclusion.teammate_conflicts.as_ref().unwrap();
        assert_eq!(teammate.combinations.len(), 3);
        assert_eq!(config.synergies.len(), 4);
    }

    /// 嵌入默认必须与仓库中的 YAML 一致：改表必须同步改动才算生效，
    /// 否则无源码 / 文件缺失环境下回退到的默认值会与设计师预期不一致。
    #[test]
    fn test_embedded_default_parses_and_matches_yaml() {
        let embedded: ElementConfig = serde_yaml::from_str(ELEMENT_CONFIG_EMBEDDED)
            .expect("嵌入默认应可解析");
        let file = element_config_path().expect("随包YAML应存在");
        let from_file = load_element_config_from(&file).expect("随包YAML应可解析");
        assert_eq!(embedded, from_file, "嵌入默认与 src/config/element_reactions.yaml 不一致");
    }

    #[test]
    fn test_missing_file_is_error() {
        let err = load_element_config_from(Path::new("no/such/file.yaml"));
        assert!(err.is_err());
    }

    /// 配置表驱动：改 YAML 数值，运行时行为必须跟着变
    #[test]
    fn test_yaml_multiplier_drives_runtime() {
        let yaml = "reactions:\n  - target_state: Wet\n    incoming_element: Fire\n    result: Vaporize\n    damage_multiplier: 9.9\n";
        let config: ElementConfig = serde_yaml::from_str(yaml).unwrap();
        let system = ElementSystem::new(config);
        let multiplier = system.calculate_damage_multiplier(&EntityElementState::Wet, &ElementType::Fire);
        assert!((multiplier - 9.9).abs() < f32::EPSILON);
    }

    /// 互斥半径/上限也必须由配置驱动（此前硬编码在代码里，改表无效）
    #[test]
    fn test_yaml_mutual_exclusion_drives_penalty() {
        let yaml = "reactions: []\nmutual_exclusion:\n  self_conflicts:\n    - elements: [IceArmor, FireWeapon]\n      effect: fire_output_reduction\n      value: 0.6\n  teammate_conflicts:\n    radius: 10.0\n    falloff: linear\n    combinations:\n      - elements: [IceArmor, FireWeapon]\n        effect: fire_output_reduction\n        max_value: 0.5\n";
        let config: ElementConfig = serde_yaml::from_str(yaml).unwrap();
        let system = ElementSystem::new(config);

        // 队友互斥：半径10米、上限50%（贴脸）
        let penalty = system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 0.0);
        assert!((penalty - 0.5).abs() < 1e-3);
        // 5米处：旧硬编码半径下是0，现在 0.5 * (1 - 5/10) = 0.25
        let penalty = system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 5.0);
        assert!((penalty - 0.25).abs() < 1e-3);
        // 10米外无惩罚
        assert_eq!(system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 10.0), 0.0);

        // 自身互斥：读 YAML 的 value
        let penalty = system.calculate_self_penalty(&ElementType::Ice, &ElementType::Fire);
        assert!((penalty - 0.6).abs() < f32::EPSILON);
    }

    /// 环境修正必须来自配置表（此前硬编码，YAML 环境行不生效）
    #[test]
    fn test_environment_modifier_from_config() {
        let yaml = "reactions:\n  - target_state: RainEnvironment\n    incoming_element: Fire\n    result: RainSuppressed\n    damage_multiplier: 0.3\n";
        let config: ElementConfig = serde_yaml::from_str(yaml).unwrap();
        let system = ElementSystem::new(config);
        let modifier = system.get_environment_modifier(&EntityElementState::RainEnvironment, &ElementType::Fire);
        assert!((modifier - 0.3).abs() < f32::EPSILON);
        // 表中没有的环境组合返回 1.0
        let modifier = system.get_environment_modifier(&EntityElementState::SnowEnvironment, &ElementType::Fire);
        assert!((modifier - 1.0).abs() < f32::EPSILON);
    }
}
