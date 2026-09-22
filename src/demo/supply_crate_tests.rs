//! 物资箱生成单元验证（headless bevy，不启动窗口）
//!
//! 目的：确认 [`spawn_crates`] 并入 `setup_world` 后确实生成固定数量的
//! [`SupplyCrate`] 实体。装箱时运行 `cargo test --features demo`。

use bevy::prelude::*;
use super::supply_crate::*;

fn call_spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_crates(&mut commands, &mut meshes, &mut materials);
}

#[test]
fn spawn_crates_creates_four_crates() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(AssetPlugin::default());
    app.world_mut().init_resource::<Assets<Mesh>>();
    app.world_mut().init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, call_spawn);
    app.update();

    let n = app.world_mut().query::<&SupplyCrate>().iter(app.world_mut()).count();
    assert_eq!(n, 4, "spawn_crates 应生成 4 只木箱，实际 {n}");
}

#[test]
fn crate_cells_is_twelve() {
    assert_eq!(CRATE_CELLS, 12, "物资箱应为 3×4=12 格");
    assert_eq!(CRATE_SLOTS, 8, "背包落点格数应为 8");
}