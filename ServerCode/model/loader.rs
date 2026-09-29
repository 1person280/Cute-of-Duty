//! 体素模型规格加载器（YSM「是，史蒂夫模型」线格式 → 可下行的稳定结构）
//!
//! 设计动机（对齐全局约束「易变内容放服务端、客户端只画」）：模型几何是**会随造型迭代
//! 而变**的易变内容，故其源文件（单份 `FireFox.json`）与解析都落在服务端 `model` 模块；
//! 客户端不持有几何，只消费服务端经协议下发的 [`VoxelModelSpec`] / [`VoxelAnimationSpec`]
//! 内存副本。
//!
//! 单一事实来源：几何与动画**合并为同一份** `FireFox.json`——几何在顶层
//! `minecraft:geometry`、动画在顶层 `animations`，两段同源同版本，避免造型与动作分居两文件
//! 后各自漂移。以编译期 `include_str!` 嵌入（与 `config` 模块同一约定），改造型只改 JSON、
//! 重新编译即生效，不会与代码里的硬编码漂移。
//!
//! ## 命名免责声明
//! 本模块与源文件 `FireFox.json` 中的 "FireFox" 指本作我方火系干员「焰狐」（Flame Fox）；
//! 仅为本作角色建模命名，与 Mozilla 基金会的 Mozilla Firefox 浏览器**没有任何关系**。
//!
//! 坐标系约定沿用设计像素空间（角点原点、脸朝 -Z），像素→米的比例 [`YANHU_SCALE`] 随
//! spec 下发，客户端据此换算，无需自己猜测尺度。

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

// 形状契约类型由契约 crate 提供（随协议下行给客户端）；本模块只负责把 YSM 线格式
// 解析成这些结构。
use cute_of_duty_contract::model::{
    RotationExpr, VoxelAnimationSpec, VoxelBone, VoxelBoneTrack, VoxelCube, VoxelModelSpec,
    YANHU_SCALE,
};

use super::ModelPreset;

/// 编译期嵌入的焰狐模型源文件（几何 + 动画合并，权威默认，随二进制分发；无源码环境亦一致）。
pub const FIREFOX_MODEL: &str = include_str!("FireFox.json");

// —— 原始线格式（仅解析用，不外泄）——
// 几何与动画同处一份 JSON，故用单一 `RawModelFile` 承载两段，各自再投影为下行结构。

#[derive(Deserialize)]
struct RawModelFile {
    #[serde(rename = "minecraft:geometry")]
    geometries: Vec<RawGeometryEntry>,
    animations: HashMap<String, RawClip>,
}

#[derive(Deserialize)]
struct RawGeometryEntry {
    bones: Vec<RawBone>,
}

#[derive(Deserialize)]
struct RawBone {
    name: String,
    parent: Option<String>,
    pivot: [f32; 3],
    #[serde(default)]
    rotation: [f32; 3],
    cubes: Vec<RawCube>,
}

#[derive(Deserialize)]
struct RawCube {
    origin: [f32; 3],
    size: [f32; 3],
    /// 逐盒材质键（本格式相对基岩版几何的自有扩展；未标注时按骨名兜底着色）。
    #[serde(default)]
    mat: Option<String>,
}

#[derive(Deserialize)]
struct RawClip {
    #[serde(default)]
    animation_length: f32,
    bones: HashMap<String, RawBoneAnim>,
}

#[derive(Deserialize)]
struct RawBoneAnim {
    rotation: Option<[RotationExpr; 3]>,
}

/// 服务端权威模型目录：当前仅 `OperativeFire`（我方火系干员 = 焰狐）拥有专属体素造型。
///
/// 其余 preset 客户端仍按方块回退绘制（见 `HostCode/world/model.rs`），故不入目录。
pub fn catalog() -> Vec<VoxelModelSpec> {
    static CACHE: OnceLock<Vec<VoxelModelSpec>> = OnceLock::new();
    CACHE
        .get_or_init(|| build_catalog(load_raw().geometries))
        .clone()
}

/// 服务端权威动画目录（随模型目录一并下发）。
pub fn animations() -> Vec<VoxelAnimationSpec> {
    static CACHE: OnceLock<Vec<VoxelAnimationSpec>> = OnceLock::new();
    CACHE
        .get_or_init(|| build_animations(load_raw().animations))
        .clone()
}

/// 解析编译期嵌入的模型源文件（几何 + 动画同一份 JSON）。
fn load_raw() -> RawModelFile {
    serde_json::from_str(FIREFOX_MODEL).expect("嵌入的 FireFox.json 应可解析")
}

fn build_catalog(geometries: Vec<RawGeometryEntry>) -> Vec<VoxelModelSpec> {
    let bones = geometries
        .into_iter()
        .next()
        .expect("几何文件应含至少一段 geometry")
        .bones
        .into_iter()
        .map(|b| VoxelBone {
            name: b.name,
            parent: b.parent,
            pivot: b.pivot,
            rotation: b.rotation,
            cubes: b
                .cubes
                .into_iter()
                .map(|c| VoxelCube {
                    origin: c.origin,
                    size: c.size,
                    mat: c.mat,
                })
                .collect(),
        })
        .collect();
    vec![VoxelModelSpec {
        preset: ModelPreset::OperativeFire,
        scale: YANHU_SCALE,
        bones,
    }]
}

fn build_animations(clips: HashMap<String, RawClip>) -> Vec<VoxelAnimationSpec> {
    let mut specs: Vec<VoxelAnimationSpec> = clips
        .into_iter()
        .map(|(clip, raw)| VoxelAnimationSpec {
            clip,
            length: raw.animation_length,
            tracks: raw
                .bones
                .into_iter()
                .filter_map(|(bone, anim)| {
                    anim.rotation.map(|rotation| VoxelBoneTrack { bone, rotation })
                })
                .collect(),
        })
        .collect();
    // 确定性顺序：HashMap 遍历无序，按 clip 名排序保证快照/测试稳定。
    specs.sort_by(|a, b| a.clip.cmp(&b.clip));
    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 嵌入的几何必须可解析，且骨/盒数量与 JSON 规格一致（造型改档时此测试先失败）。
    #[test]
    fn firefox_geometry_parses() {
        let models = catalog();
        assert_eq!(models.len(), 1);
        let m = &models[0];
        assert_eq!(m.preset, ModelPreset::OperativeFire);
        assert_eq!(m.scale, YANHU_SCALE);
        assert_eq!(m.bones.len(), 12, "焰狐应为 12 根骨");
        let cubes: usize = m.bones.iter().map(|b| b.cubes.len()).sum();
        assert_eq!(cubes, 43, "焰狐应为 43 个盒子");
        // 逐盒材质必须齐全：缺 `mat` 会退回按骨名着色，模型立刻失去层次（造型改档漏标注即失败）。
        for b in &m.bones {
            for c in &b.cubes {
                assert!(c.mat.is_some(), "骨 `{}` 有盒子未标注 mat", b.name);
            }
        }
        // 左右不得镜像颠倒：角色面朝 -Z、右手在 +X，故 arm_right/leg_right 必须落在 +X 侧。
        let right_arm = m.bones.iter().find(|b| b.name == "arm_right").unwrap();
        assert!(right_arm.pivot[0] > 0.0, "arm_right 应在 +X 侧（角色右手）");
        let left_arm = m.bones.iter().find(|b| b.name == "arm_left").unwrap();
        assert!(left_arm.pivot[0] < 0.0, "arm_left 应在 -X 侧（角色左手）");
        let right_leg = m.bones.iter().find(|b| b.name == "leg_right").unwrap();
        assert!(right_leg.pivot[0] > 0.0, "leg_right 应在 +X 侧");
        // 父子关系抽查：头挂躯干、尾链逐节相承、枪挂右臂。
        let head = m.bones.iter().find(|b| b.name == "head").unwrap();
        assert_eq!(head.parent.as_deref(), Some("body"));
        let tail_c = m.bones.iter().find(|b| b.name == "tail_c").unwrap();
        assert_eq!(tail_c.parent.as_deref(), Some("tail_b"));
        let gun = m.bones.iter().find(|b| b.name == "gun").unwrap();
        assert_eq!(gun.parent.as_deref(), Some("arm_right"));
    }

    /// 嵌入的动画必须可解析：三段 clip（idle/walk/hold_gun），且表达式/常量并存。
    #[test]
    fn firefox_animation_parses() {
        let clips = animations();
        assert_eq!(clips.len(), 3);
        let names: Vec<&str> = clips.iter().map(|c| c.clip.as_str()).collect();
        assert!(names.contains(&"animation.firefox.idle"));
        assert!(names.contains(&"animation.firefox.walk"));
        assert!(names.contains(&"animation.firefox.hold_gun"));

        let idle = clips
            .iter()
            .find(|c| c.clip == "animation.firefox.idle")
            .unwrap();
        assert!(idle.length > 0.0);
        let body = idle.tracks.iter().find(|t| t.bone == "body").unwrap();
        // 第一轴为表达式、后两轴为常量 0。
        assert!(matches!(body.rotation[0], RotationExpr::Expr(_)));
        assert_eq!(body.rotation[1], RotationExpr::Const(0.0));
    }

    /// 目录身份稳定：仅 OperativeFire 入目录（其余走客户端方块回退）。
    #[test]
    fn catalog_only_covers_operative_fire() {
        let models = catalog();
        assert!(models.iter().any(|m| m.preset == ModelPreset::OperativeFire));
        assert_eq!(models.len(), 1);
    }
}